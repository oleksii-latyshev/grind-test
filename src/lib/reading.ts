import { useCallback, useLayoutEffect, useState } from "react";

/** Body text sizes in px, the way a reader app offers them: a few deliberate steps. */
export const READING_SIZES = [15, 16, 17, 19, 21, 24] as const;

const DEFAULT_INDEX = 1;
const STORAGE_KEY = "grind:reading-size";
const CSS_VARIABLE = "--reading-font-size";

function readStoredIndex(): number {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === null) return DEFAULT_INDEX;
    const index = Number.parseInt(raw, 10);
    return Number.isInteger(index) && index >= 0 && index < READING_SIZES.length
      ? index
      : DEFAULT_INDEX;
  } catch {
    // Private mode, or storage blocked outright: the default is a fine answer.
    return DEFAULT_INDEX;
  }
}

/**
 * Reading text size, persisted per machine and applied as a CSS variable so every
 * `.prose-note` on the page follows it — the notes are the only thing it should resize,
 * not the UI chrome around them.
 */
export function useReadingSize() {
  const [index, setIndex] = useState(readStoredIndex);

  // Layout effect, so the note never paints once at the old size and then jumps.
  useLayoutEffect(() => {
    document.documentElement.style.setProperty(CSS_VARIABLE, `${READING_SIZES[index]}px`);
    try {
      window.localStorage.setItem(STORAGE_KEY, String(index));
    } catch {
      // Not being able to remember the choice is not a reason to refuse to apply it.
    }
  }, [index]);

  const step = useCallback((delta: number) => {
    setIndex((current) => Math.min(READING_SIZES.length - 1, Math.max(0, current + delta)));
  }, []);

  return {
    size: READING_SIZES[index],
    canDecrease: index > 0,
    canIncrease: index < READING_SIZES.length - 1,
    decrease: () => step(-1),
    increase: () => step(1),
  };
}
