//! Resolves the hardware keys behind the Linux paste chords from the active keymap.
//!
//! ydotool and dotool inject evdev keycodes through `/dev/uinput`, below XKB, so the
//! compositor applies the user's layout and options to whatever Handy sends. A fixed
//! `KEY_LEFTCTRL` (29) is Alt under `ctrl:swap_lalt_lctl` and Caps Lock under
//! `ctrl:swapcaps`, and a fixed `KEY_V` (47) is `k` under Dvorak. This module asks GDK
//! which keys currently produce `Control_L`, `Shift_L`, `Insert` and `v`, and caches the
//! answer for the paste code. When resolution fails, the standard QWERTY codes are kept,
//! so standard layouts behave exactly as before.

use log::{debug, info, warn};
use std::sync::RwLock;

/// evdev keycodes for the keys that make up the paste chords.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PasteKeycodes {
    pub ctrl: u32,
    pub shift: u32,
    pub insert: u32,
    pub v: u32,
}

impl PasteKeycodes {
    /// `KEY_LEFTCTRL`, `KEY_LEFTSHIFT`, `KEY_INSERT`, `KEY_V`: the codes Handy always sent.
    pub const DEFAULT: Self = Self {
        ctrl: 29,
        shift: 42,
        insert: 110,
        v: 47,
    };
}

impl Default for PasteKeycodes {
    fn default() -> Self {
        Self::DEFAULT
    }
}

static PASTE_KEYCODES: RwLock<PasteKeycodes> = RwLock::new(PasteKeycodes::DEFAULT);

/// The paste chord keycodes for the active keymap (the defaults until resolved).
pub fn paste_keycodes() -> PasteKeycodes {
    PASTE_KEYCODES
        .read()
        .map(|codes| *codes)
        .unwrap_or(PasteKeycodes::DEFAULT)
}

fn store(codes: PasteKeycodes) -> bool {
    match PASTE_KEYCODES.write() {
        Ok(mut current) => {
            let changed = *current != codes;
            *current = codes;
            changed
        }
        Err(_) => false,
    }
}

/// One keymap entry that produces a keysym: an evdev keycode with its XKB group and level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyCandidate {
    pub evdev: u32,
    pub group: i32,
    pub level: i32,
}

/// Picks the evdev keycode to send for one keysym.
///
/// Only the lowest level that has any entry counts (level 0 means no Shift or AltGr is
/// needed), and within that the lowest group (group 0, the first layout). Among those
/// keys the default code wins if it is one of them, because the key Handy always sent
/// still works and there is nothing to change; `ctrl:nocaps`, for example, adds Caps
/// Lock as a second Control_L and leaves left Ctrl alone. Otherwise the lowest keycode
/// wins, and no candidates at all gives the default.
pub fn select_keycode(candidates: &[KeyCandidate], default: u32) -> u32 {
    let Some(level) = candidates.iter().map(|c| c.level).filter(|l| *l >= 0).min() else {
        return default;
    };
    let Some(group) = candidates
        .iter()
        .filter(|c| c.level == level)
        .map(|c| c.group)
        .min()
    else {
        return default;
    };
    let mut preferred: Vec<u32> = candidates
        .iter()
        .filter(|c| c.level == level && c.group == group)
        .map(|c| c.evdev)
        .collect();
    if preferred.contains(&default) {
        return default;
    }
    preferred.sort_unstable();
    preferred.first().copied().unwrap_or(default)
}

/// Picks a keycode from the first keysym in `keysyms` that has any entry, so a keymap
/// without `Control_L` can still fall back to `Control_R`.
fn select_from_first_present(candidates_per_keysym: &[Vec<KeyCandidate>], default: u32) -> u32 {
    candidates_per_keysym
        .iter()
        .find(|candidates| !candidates.is_empty())
        .map(|candidates| select_keycode(candidates, default))
        .unwrap_or(default)
}

/// GDK hardware keycodes are X keycodes on both the X11 and Wayland backends, which are
/// the evdev keycode plus 8.
const X_KEYCODE_OFFSET: u32 = 8;

fn candidates_for_keyval(keymap: &gtk::gdk::Keymap, keyval: u32) -> Vec<KeyCandidate> {
    keymap
        .entries_for_keyval(keyval)
        .into_iter()
        .filter(|entry| entry.keycode() > X_KEYCODE_OFFSET)
        .map(|entry| KeyCandidate {
            evdev: entry.keycode() - X_KEYCODE_OFFSET,
            group: entry.group(),
            level: entry.level(),
        })
        .collect()
}

/// Reads the paste keycodes from the default display's keymap. Must run on the GTK main
/// thread after GTK is initialized. Returns `None` when there is no display or keymap.
pub fn resolve_from_gdk() -> Option<PasteKeycodes> {
    if !gtk::is_initialized_main_thread() {
        return None;
    }
    let display = gtk::gdk::Display::default()?;
    let keymap = gtk::gdk::Keymap::for_display(&display)?;
    Some(resolve_from_keymap(&keymap))
}

fn resolve_from_keymap(keymap: &gtk::gdk::Keymap) -> PasteKeycodes {
    use gtk::gdk::keys::constants as keys;

    let lookup = |keysyms: &[gtk::gdk::keys::Key], default: u32| {
        let candidates: Vec<Vec<KeyCandidate>> = keysyms
            .iter()
            .map(|keysym| candidates_for_keyval(keymap, **keysym))
            .collect();
        select_from_first_present(&candidates, default)
    };
    let defaults = PasteKeycodes::DEFAULT;

    PasteKeycodes {
        ctrl: lookup(&[keys::Control_L, keys::Control_R], defaults.ctrl),
        shift: lookup(&[keys::Shift_L, keys::Shift_R], defaults.shift),
        insert: lookup(&[keys::Insert], defaults.insert),
        v: lookup(&[keys::v], defaults.v),
    }
}

fn refresh(reason: &str) {
    match resolve_from_gdk() {
        Some(codes) => {
            let changed = store(codes);
            if changed || reason == "startup" {
                info!(
                    "Linux paste keycodes from the active keymap ({reason}): ctrl={} shift={} insert={} v={}",
                    codes.ctrl, codes.shift, codes.insert, codes.v
                );
            } else {
                debug!("Linux keymap changed ({reason}); paste keycodes unchanged");
            }
        }
        None => {
            let codes = paste_keycodes();
            warn!(
                "Could not read the active keymap ({reason}); paste keycodes stay ctrl={} shift={} insert={} v={}",
                codes.ctrl, codes.shift, codes.insert, codes.v
            );
        }
    }
}

/// Resolves the paste keycodes now and again whenever the keymap changes (a new layout,
/// an XKB option toggled in the desktop settings, or, on Wayland, the compositor sending
/// the real keymap after startup). Call once from the GTK main thread.
pub fn init() {
    refresh("startup");
    if !gtk::is_initialized_main_thread() {
        return;
    }
    if let Some(keymap) =
        gtk::gdk::Display::default().and_then(|display| gtk::gdk::Keymap::for_display(&display))
    {
        keymap.connect_keys_changed(|_| refresh("keymap changed"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(evdev: u32, group: i32, level: i32) -> KeyCandidate {
        KeyCandidate {
            evdev,
            group,
            level,
        }
    }

    #[test]
    fn no_candidates_keep_the_default() {
        assert_eq!(select_keycode(&[], 29), 29);
        assert_eq!(select_from_first_present(&[vec![], vec![]], 29), 29);
    }

    #[test]
    fn standard_layout_keeps_left_ctrl() {
        assert_eq!(select_keycode(&[key(29, 0, 0)], 29), 29);
    }

    #[test]
    fn swapped_left_alt_and_ctrl_uses_the_alt_key() {
        // ctrl:swap_lalt_lctl: only the physical left Alt (56) produces Control_L.
        assert_eq!(select_keycode(&[key(56, 0, 0)], 29), 56);
    }

    #[test]
    fn caps_as_extra_ctrl_keeps_left_ctrl() {
        // ctrl:nocaps: Caps Lock (58) produces Control_L and left Ctrl still does too.
        assert_eq!(select_keycode(&[key(58, 0, 0), key(29, 0, 0)], 29), 29);
    }

    #[test]
    fn swapped_caps_and_ctrl_uses_caps_lock() {
        // ctrl:swapcaps: left Ctrl now produces Caps_Lock, so only 58 is Control_L.
        assert_eq!(select_keycode(&[key(58, 0, 0)], 29), 58);
    }

    #[test]
    fn lowest_keycode_wins_when_default_is_absent() {
        assert_eq!(select_keycode(&[key(125, 0, 0), key(58, 0, 0)], 29), 58);
    }

    #[test]
    fn level_zero_beats_shifted_entries() {
        // A key that needs Shift to produce the keysym is not a plain key press.
        assert_eq!(select_keycode(&[key(47, 0, 1), key(52, 0, 0)], 47), 52);
    }

    #[test]
    fn first_group_beats_later_groups() {
        // us(dvorak),us: v is 52 in group 0 and 47 in group 1; group 0 is what is typed.
        assert_eq!(select_keycode(&[key(47, 1, 0), key(52, 0, 0)], 47), 52);
    }

    #[test]
    fn later_group_is_used_when_the_first_has_no_entry() {
        // ru,us: group 0 has no v at all, the us group has it on 47.
        assert_eq!(select_keycode(&[key(47, 1, 0)], 47), 47);
    }

    #[test]
    fn control_r_is_used_only_when_control_l_is_missing() {
        assert_eq!(
            select_from_first_present(&[vec![], vec![key(97, 0, 0)]], 29),
            97
        );
        assert_eq!(
            select_from_first_present(&[vec![key(56, 0, 0)], vec![key(97, 0, 0)]], 29),
            56
        );
    }

    /// Prints the codes resolved from the keymap of `$DISPLAY` (or `$WAYLAND_DISPLAY`).
    /// Run with `cargo test linux_keymap::tests::print_resolved -- --ignored --nocapture`
    /// after, for example, `setxkbmap -display :99 -option ctrl:swap_lalt_lctl`.
    ///
    /// With `KEYMAP_SWITCH_CMD` set, it then runs that command (say, another
    /// `setxkbmap`), waits for GDK's `keys-changed` signal and prints the cached codes
    /// again, which exercises the refresh path `init()` installs.
    #[test]
    #[ignore]
    fn print_resolved_paste_keycodes_for_current_display() {
        use gtk::prelude::*;

        gtk::init().expect("GTK init needs a display");
        let backend = gtk::gdk::Display::default()
            .map(|d| d.type_().name().to_string())
            .unwrap_or_default();
        init();
        if let Some(keymap) =
            gtk::gdk::Display::default().and_then(|display| gtk::gdk::Keymap::for_display(&display))
        {
            use gtk::gdk::keys::constants as keys;
            for (name, keysym) in [("Control_L", keys::Control_L), ("v", keys::v)] {
                let entries: Vec<String> = candidates_for_keyval(&keymap, *keysym)
                    .iter()
                    .map(|c| format!("{}(g{} l{})", c.evdev, c.group, c.level))
                    .collect();
                println!("CANDIDATES {name}: {}", entries.join(" "));
            }
        }
        let codes = paste_keycodes();
        println!(
            "RESOLVED backend={backend} ctrl={} shift={} insert={} v={}",
            codes.ctrl, codes.shift, codes.insert, codes.v
        );
        if let Ok(expected) = std::env::var("EXPECT_PASTE_KEYCODES") {
            let actual = format!(
                "{} {} {} {}",
                codes.ctrl, codes.shift, codes.insert, codes.v
            );
            assert_eq!(actual, expected);
        }

        if let Ok(cmd) = std::env::var("KEYMAP_SWITCH_CMD") {
            let before = paste_keycodes();
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .status()
                .expect("run KEYMAP_SWITCH_CMD");
            assert!(status.success());
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let context = gtk::glib::MainContext::default();
            while paste_keycodes() == before && std::time::Instant::now() < deadline {
                context.iteration(false);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let codes = paste_keycodes();
            println!(
                "AFTER-SWITCH ctrl={} shift={} insert={} v={}",
                codes.ctrl, codes.shift, codes.insert, codes.v
            );
        }
    }
}
