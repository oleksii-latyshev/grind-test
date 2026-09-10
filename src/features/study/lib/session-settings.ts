import { usePreference } from '@/lib/prefs';
import type { SessionMode } from '@/lib/types';
import { MAX_TOPICS, SESSION_SIZES } from './study';

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
