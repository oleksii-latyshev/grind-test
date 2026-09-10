import { describe, expect, test } from 'bun:test';

import type { Translate } from '@/i18n';
import type { TopicStudy } from '@/lib/types';
import { formatDue, formatHours, scoreTone, stageOf } from './study';

const t: Translate = (id, values) => (values ? `${id} ${JSON.stringify(values)}` : id);

const entry = (level: number, readCount = 0) => ({ level, read_count: readCount }) as TopicStudy;

describe('stageOf', () => {
  test('climbs with the ladder level', () => {
    expect(stageOf(undefined)).toBe('new');
    expect(stageOf(entry(0))).toBe('new');
    expect(stageOf(entry(0, 1))).toBe('reading');
    expect([1, 2].map((level) => stageOf(entry(level)))).toEqual(['learning', 'learning']);
    expect([3, 4].map((level) => stageOf(entry(level)))).toEqual(['review', 'review']);
    expect([5, 6].map((level) => stageOf(entry(level)))).toEqual(['mastered', 'mastered']);
  });
});

test('scoreTone splits at the pass and hold thresholds', () => {
  expect(scoreTone(80)).toBe('text-primary');
  expect(scoreTone(79)).toBe('text-chart-3');
  expect(scoreTone(60)).toBe('text-chart-3');
  expect(scoreTone(59)).toBe('text-destructive');
});

test('formatDue counts whole days from now', () => {
  const inDays = (days: number) => new Date(Date.now() + days * 86_400_000).toISOString();
  expect(formatDue(t, null)).toBe('due.none');
  expect(formatDue(t, inDays(-3))).toBe('due.today');
  expect(formatDue(t, inDays(1))).toBe('due.tomorrow');
  expect(formatDue(t, inDays(4))).toBe('due.days {"days":4}');
});

test('formatHours switches units at an hour and drops the decimal past ten', () => {
  expect(formatHours(t, 45)).toBe('time.minutes {"value":45}');
  expect(formatHours(t, 90)).toBe('time.hours {"value":"1.5"}');
  expect(formatHours(t, 750)).toBe('time.hours {"value":13}');
});
