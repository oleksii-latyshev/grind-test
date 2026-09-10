import { useCallback, useState } from 'react';

import { MAX_TOPICS, SESSION_SIZES } from './study';
import type { SessionMode } from './types';

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

/** How topics are chosen, as the study tab offers it. */
export type TopicPick = 'auto' | 'spread' | 'manual';

/**
 * The three choices at the top of a new session. Kept together under one key so
 * "continue with the same settings" can read back exactly what the tab is showing.
 *
 * The manually chosen topic ids are deliberately not part of this: they belong to one
 * subject and one sitting, and restoring them a week later would be noise.
 */
export interface SessionSettings {
  pace: SessionMode;
  pick: TopicPick;
  size: number;
}

const KEY = 'grind:session-settings';

export const DEFAULT_SESSION_SETTINGS: SessionSettings = {
  pace: 'sprint',
  pick: 'auto',
  size: SESSION_SIZES.sprint[1],
};

export function isSessionSettings(value: unknown): value is SessionSettings {
  if (!value || typeof value !== 'object') return false;
  const { pace, pick, size } = value as Partial<SessionSettings>;
  return (
    (pace === 'full' || pace === 'balanced' || pace === 'sprint') &&
    (pick === 'auto' || pick === 'spread' || pick === 'manual') &&
    typeof size === 'number' &&
    Number.isInteger(size) &&
    size >= 1 &&
    // The size steps differ per pace, so a stored pair can only be trusted together.
    size <= MAX_TOPICS[pace]
  );
}

/** Read the settings outside a component — used by "continue" at the end of a session. */
export function readSessionSettings(): SessionSettings {
  try {
    const raw = window.localStorage.getItem(KEY);
    if (raw === null) return DEFAULT_SESSION_SETTINGS;
    const parsed: unknown = JSON.parse(raw);
    return isSessionSettings(parsed) ? parsed : DEFAULT_SESSION_SETTINGS;
  } catch {
    return DEFAULT_SESSION_SETTINGS;
  }
}

export function useSessionSettings() {
  return usePreference<SessionSettings>(KEY, DEFAULT_SESSION_SETTINGS, isSessionSettings);
}
