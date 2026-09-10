import { beforeEach, expect, test } from 'bun:test';

import { clearDraft, readDraft, writeDraft } from './drafts';
import {
  DEFAULT_SESSION_SETTINGS,
  isSessionSettings,
  readSessionSettings,
  type SessionSettings,
} from './session-settings';

const storage = new Map<string, string>();

beforeEach(() => {
  storage.clear();
  Object.assign(globalThis, {
    window: {
      localStorage: {
        getItem: (key: string) => storage.get(key) ?? null,
        setItem: (key: string, value: string) => storage.set(key, value),
        removeItem: (key: string) => storage.delete(key),
      },
    },
  });
});

test('a stored size is only valid for the pace it was stored with', () => {
  expect(isSessionSettings({ pace: 'sprint', pick: 'auto', size: 10 })).toBe(true);
  expect(isSessionSettings({ pace: 'full', pick: 'auto', size: 10 })).toBe(false);
  expect(isSessionSettings({ pace: 'full', pick: 'auto', size: 2.5 })).toBe(false);
  expect(isSessionSettings({ pace: 'turbo', pick: 'auto', size: 3 })).toBe(false);
  expect(isSessionSettings(null)).toBe(false);
});

test('readSessionSettings falls back on anything an older build could have left', () => {
  expect(readSessionSettings()).toEqual(DEFAULT_SESSION_SETTINGS);

  storage.set('grind:session-settings', '{not json');
  expect(readSessionSettings()).toEqual(DEFAULT_SESSION_SETTINGS);

  const saved: SessionSettings = { pace: 'balanced', pick: 'spread', size: 5 };
  storage.set('grind:session-settings', JSON.stringify(saved));
  expect(readSessionSettings()).toEqual(saved);
});

test('a draft round-trips and drops values that are not text', () => {
  writeDraft('s1', { 'demo/1.1': 'відповідь' });
  expect(readDraft('s1')).toEqual({ 'demo/1.1': 'відповідь' });

  storage.set('grind:session-draft:s2', JSON.stringify({ 'demo/1.1': 'ok', 'demo/1.2': 42 }));
  expect(readDraft('s2')).toEqual({ 'demo/1.1': 'ok' });

  clearDraft('s1');
  expect(readDraft('s1')).toEqual({});
});
