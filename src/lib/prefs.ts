import { useCallback, useState } from 'react';

/**
 * A value remembered per machine, in `localStorage`.
 *
 * `isValid` runs on every read because what is in storage was written by an older build of
 * the app: a mode that no longer exists or a size that is now out of range must not be able
 * to put the UI into a state it cannot render.
 */
export function usePreference<T>(
  key: string,
  fallback: T,
  isValid: (value: unknown) => value is T,
): [T, (value: T) => void] {
  const [value, setStored] = useState<T>(() => {
    try {
      const raw = window.localStorage.getItem(key);
      if (raw === null) return fallback;
      const parsed: unknown = JSON.parse(raw);
      return isValid(parsed) ? parsed : fallback;
    } catch {
      // Private mode, blocked storage, or a half-written value: the default is fine.
      return fallback;
    }
  });

  const set = useCallback(
    (next: T) => {
      setStored(next);
      try {
        window.localStorage.setItem(key, JSON.stringify(next));
      } catch {
        // Not being able to remember the choice is no reason to refuse to apply it.
      }
    },
    [key],
  );

  return [value, set];
}
