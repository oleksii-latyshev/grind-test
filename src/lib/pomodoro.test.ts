import { expect, test } from 'bun:test';

import { advance, DEFAULT_POMODORO, formatClock } from './pomodoro';

test('every fourth work round ends in a long break', () => {
  let work = advance({ phase: 'long', round: 0, endsAt: null, left: 0 }, DEFAULT_POMODORO);
  const phases: string[] = [];
  for (let step = 0; step < 8; step++) {
    const rest = advance(work, DEFAULT_POMODORO);
    phases.push(rest.phase);
    work = advance(rest, DEFAULT_POMODORO);
  }
  expect(phases).toEqual(['short', 'short', 'short', 'long', 'short', 'short', 'short', 'long']);
});

test('a break is followed by a paused, full-length work round', () => {
  const next = advance({ phase: 'long', round: 4, endsAt: null, left: 0 }, DEFAULT_POMODORO);
  expect(next).toEqual({ phase: 'work', round: 4, endsAt: null, left: 25 * 60_000 });
});

test('formatClock rounds to seconds and never goes negative', () => {
  expect(formatClock(25 * 60_000)).toBe('25:00');
  expect(formatClock(61_499)).toBe('1:01');
  expect(formatClock(-5)).toBe('0:00');
});
