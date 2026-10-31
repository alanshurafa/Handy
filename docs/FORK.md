# Handy fork additions

This is Alan's fork of [cjpais/Handy](https://github.com/cjpais/Handy), adding local spoken snippets and a Windows live transcript editor. It retains Handy's upstream MIT license and credits. Snippet code in this fork is newly written; the related projects below informed the feature comparison, not copied implementation.

## What this fork adds

| Feature                       | Where                             | Behavior                                                                                                                                                                         |
| ----------------------------- | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spoken snippets               | Snippets settings screen          | Map a spoken phrase such as “my work email” to an exact email address or other saved text. Add, edit, remove, and enable individual entries.                                     |
| Live transcript editing       | Live recording overlay on Windows | Click the transcript to edit while recording continues. Manual corrections take precedence over recognition updates; uncertain merges expose the latest recognition separately.  |
| Review before paste           | Live transcript editor            | **Stop & review** finishes recognition without sending text. **Paste** sends the reviewed draft to the captured destination. A reported failure keeps the draft available.       |
| Local deterministic expansion | Final transcription output        | Whole phrases match without case sensitivity; whitespace differences are ignored. Longer phrases win at the same position. Inserted text is literal and is never expanded again. |
| Runtime undo                  | Snippets toggle                   | Off by default. Turning it off immediately returns output to ordinary Handy behavior without deleting saved entries.                                                             |
| Preserve recognition history  | History                           | The original transcription is retained. Final expanded text is also stored in the existing processed-text field when it differs.                                                 |
| Protect fork installation     | Updater                           | Upstream self-updates are disabled so an official Handy release cannot silently remove this fork's features. Install fork updates manually.                                      |

Snippet expansion uses the existing dictation shortcut and runs locally. Phrase matching is deterministic; it does not use fuzzy autocorrect or regex rules.

## Using snippets

1. Open **Snippets** from the sidebar.
2. Add a distinct phrase, for example `my work email`, and replacement text, for example `you@example.com`.
3. Turn **Snippets** on. Each row can also be disabled separately.
4. Dictate the phrase, either alone or in a sentence. Expansion happens after recording ends and after optional AI post-processing. The live overlay may still show the spoken phrase.

In sentences, punctuation around a phrase is preserved. When the entire transcription is one trigger, automatic trailing sentence punctuation is ignored: saying “my work email” inserts the saved address exactly, without an extra period. A phrase must be recognized correctly to match: this is not fuzzy matching. Avoid triggers you regularly say in ordinary conversation. Replacement text supports line breaks; punctuation and capitalization in the saved replacement are preserved. Limits: 500 entries, 100 characters per phrase, 4,000 per replacement. Duplicate phrases differing only in case or whitespace are rejected. A shorter trigger still applies if a longer overlapping trigger is disabled.

Disabling Snippets restores ordinary behavior for subsequent transcriptions. To return to upstream Handy, reinstall the official application; its model and settings directories are unchanged. Back up settings before switching versions, since upstream may discard fields it does not recognize.

Normal paste preferences still apply. If you do not want a space added after an email address, turn off **Advanced → Append Trailing Space**. Snippets do not change that preference.

## Editing a live transcript (Windows)

1. Start dictation with the live transcript overlay enabled, leaving the destination text field active.
2. Click the transcript in the overlay. Recording continues while you edit, and new speech is merged into the draft. Your manual corrections take precedence.
3. Choose **Stop & review** or press **Ctrl+Enter** to stop recording and wait for the remaining speech to finish transcribing. The normal stop shortcut also leaves an open editor ready for review.
4. Read the final draft. If recognition could not be merged safely, **Latest recognition** appears separately so you can copy any missing words into your draft.
5. Choose **Paste** or press **Ctrl+Enter** again to send the reviewed draft. Enter alone inserts a line break. Moving focus away does not paste.

**Cancel** or **Escape** discards the editor draft without pasting. Ordinary dictation keeps its existing behavior when you do not open the editor.

Paste tries to restore the destination active before you opened the editor, with the recording-start destination as a fallback. If that destination cannot be restored or paste reports an error, the editor keeps your draft so you can copy it manually or retry. Check the destination before retrying to avoid duplicate text if the first attempt reached it.

The editor is Windows-first. On macOS and Linux, the live transcript remains read-only and explains that editing is currently available on Windows. Recognition can revise earlier words as well as append new ones; review the finalized draft before sending it. The browser fixture does not verify microphone capture, native focus restoration, or delivery into another application.

## Other forks worth inspecting

Direct forks inspected October 31, 2026; stars are a snapshot, not a quality ranking.

| Fork                                                                                                   | Stars | Verified additions relevant to comparison                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------ | ----: | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Parler](https://github.com/Melvynx/Parler)                                                            |   237 | AI prompt actions and per-action shortcuts, saved models, Gemini integration, history reprocessing. No spoken snippets found in inspected material.                           |
| [AIVORelay](https://github.com/MaxITService/AIVORelay#-text-replacement)                               |    59 | Text find/replace with case options, regex capture groups and escaped line breaks; application profiles and broader voice tools. Closest existing full replacement interface. |
| [klava-nevinovata](https://github.com/egsok/klava-nevinovata)                                          |    37 | Russian recognition improvements, clipboard handling, Windows hotkey recovery.                                                                                                |
| [anxkhn/Handy](https://github.com/anxkhn/Handy#key-changes-in-this-fork)                               |    23 | Apple Silicon/Core ML acceleration and encoder downloads.                                                                                                                     |
| [easychen/Handy](https://github.com/easychen/Handy/blob/main/src/components/settings/RegexFilters.tsx) |    15 | Configurable sequential regex replacement rules in the transcription pipeline.                                                                                                |
| [Tolkovo](https://github.com/kozliq/Tolkovo/blob/main/src-tauri/src/tolkovo/dictionary.rs)             |    13 | Whole-word and phrase dictionary replacements with longest-match ordering, flexible whitespace, and Russian language handling.                                                |

Those related replacement implementations retain MIT licenses. Any future code reuse must preserve their applicable copyright and permission notices.

## Building

Follow [BUILD.md](../BUILD.md) for prerequisites. Fork builds produce unsigned installers and do not create upstream updater artifacts or use the upstream developer's signing service. This fork does not provide signed automatic updates.

Windows x64 builds can use `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts\build-fork.ps1`. It runs at idle priority, limits compilation to four logical processors and two parallel Cargo jobs, and builds the NSIS installer. This keeps compilation from taking over the machine used for dictation. Optional `-RuntimeDirectory` accepts an existing speech-runtime install prefix.

On this Windows build machine, the speech runtime's generated junction directory failed CMake's temporary compiler probes. A direct build of the same `transcribe-cpp-sys` 0.4.0 sources in a short directory succeeded. The supported `TRANSCRIBE_DIR` environment variable points Cargo at that staged runtime. It retains the dynamic CPU backends and Vulkan features selected by Cargo; no speech-engine source was changed.

## Verification

Windows x64 verification for `0.9.8-snippets.1`:

- TypeScript, ESLint, Rust formatting and Clippy passed. Clippy reports existing upstream warnings.
- Rust library tests: 324 passed, 18 ignored. Browser tests: 5 passed; snippet editor tests use in-memory persistence fixtures.
- The optimized Windows NSIS installer built successfully.
- The compiled app transcribed synthetic speech with Parakeet Unified on CPU. A sentence expanded to `Please send this to snippet-proof@example.com.`; a standalone trigger produced exactly `snippet-proof@example.com`; disabling snippets restored the recognized phrase.
- Physical microphone dictation and non-Windows builds were not tested in this change.
