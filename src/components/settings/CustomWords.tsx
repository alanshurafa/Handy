import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { notify } from "@/stores/noticeStore";
import { useSettings } from "../../hooks/useSettings";
import { isComposingKeyEvent } from "../../lib/utils/keyboard";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface CustomWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

type SortMode = "date-asc" | "date-desc" | "alpha-asc" | "alpha-desc";

const SORT_STORAGE_KEY = "handy.customWords.sort";
const SORT_MODES: SortMode[] = [
  "date-asc",
  "date-desc",
  "alpha-asc",
  "alpha-desc",
];

// Display order only: the saved list keeps the order words were added, which
// is the only "date" information custom words have.
const readSortMode = (): SortMode => {
  try {
    const saved = localStorage.getItem(SORT_STORAGE_KEY) as SortMode | null;
    return saved && SORT_MODES.includes(saved) ? saved : "date-asc";
  } catch {
    return "date-asc";
  }
};

const normalizeCustomWord = (word: string) =>
  word
    .replace(/[<>"']/g, "")
    .replace(/\s+/g, " ")
    .trim();

export const CustomWords: React.FC<CustomWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [newWord, setNewWord] = useState("");
    const customWords = getSetting("custom_words") || [];
    const normalizedWord = normalizeCustomWord(newWord);
    const [sortMode, setSortMode] = useState<SortMode>(readSortMode);

    const sortedWords = useMemo(() => {
      switch (sortMode) {
        case "date-desc":
          return [...customWords].reverse();
        case "alpha-asc":
          return [...customWords].sort((a, b) =>
            a.localeCompare(b, undefined, { sensitivity: "base" }),
          );
        case "alpha-desc":
          return [...customWords].sort((a, b) =>
            b.localeCompare(a, undefined, { sensitivity: "base" }),
          );
        default:
          return customWords;
      }
    }, [customWords, sortMode]);

    const changeSortMode = (mode: SortMode) => {
      setSortMode(mode);
      try {
        localStorage.setItem(SORT_STORAGE_KEY, mode);
      } catch {
        // Remembering the sort is a convenience; ignore storage failures.
      }
    };

    const isAlphaSort = sortMode.startsWith("alpha");
    const handleAlphaSort = () =>
      changeSortMode(sortMode === "alpha-asc" ? "alpha-desc" : "alpha-asc");
    const handleDateSort = () =>
      changeSortMode(sortMode === "date-asc" ? "date-desc" : "date-asc");

    const handleAddWord = () => {
      if (normalizedWord && normalizedWord.length <= 50) {
        if (customWords.includes(normalizedWord)) {
          notify({
            title: t("settings.advanced.customWords.duplicate", {
              word: normalizedWord,
            }),
          });
          return;
        }
        updateSetting("custom_words", [...customWords, normalizedWord]);
        setNewWord("");
      }
    };

    const handleRemoveWord = (wordToRemove: string) => {
      updateSetting(
        "custom_words",
        customWords.filter((word) => word !== wordToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (isComposingKeyEvent(e.nativeEvent)) return;
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddWord();
      }
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.customWords.title")}
          description={t("settings.advanced.customWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-40"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t("settings.advanced.customWords.placeholder")}
              variant="compact"
              disabled={isUpdating("custom_words")}
            />
            <Button
              onClick={handleAddWord}
              disabled={
                !normalizedWord ||
                normalizedWord.length > 50 ||
                isUpdating("custom_words")
              }
              variant="primary"
              size="md"
            >
              {t("settings.advanced.customWords.add")}
            </Button>
            <Button
              onClick={handleAlphaSort}
              disabled={customWords.length < 2}
              variant={isAlphaSort ? "primary-soft" : "secondary"}
              size="md"
              aria-pressed={isAlphaSort}
              title={t("settings.advanced.customWords.sortAlphaTitle")}
            >
              {sortMode === "alpha-desc"
                ? t("settings.advanced.customWords.sortAlphaDesc")
                : t("settings.advanced.customWords.sortAlpha")}
            </Button>
            <Button
              onClick={handleDateSort}
              disabled={customWords.length < 2}
              variant={isAlphaSort ? "secondary" : "primary-soft"}
              size="md"
              aria-pressed={!isAlphaSort}
              title={t("settings.advanced.customWords.sortDateTitle")}
            >
              {sortMode === "date-desc"
                ? t("settings.advanced.customWords.sortNewest")
                : t("settings.advanced.customWords.sortOldest")}
            </Button>
          </div>
        </SettingContainer>
        {customWords.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-line"} flex flex-wrap gap-1`}
          >
            {sortedWords.map((word) => (
              <Button
                key={word}
                onClick={() => handleRemoveWord(word)}
                disabled={isUpdating("custom_words")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.customWords.remove", { word })}
              >
                <span>{word}</span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </>
    );
  },
);
