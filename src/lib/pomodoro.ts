import { useCallback, useEffect, useRef, useState } from 'react';

import { usePreference } from './prefs';

export type Phase = 'work' | 'short' | 'long';

export interface PomodoroSettings {
  /** Minutes. */
  work: number;
  short: number;
  long: number;
  /** A long break after this many work rounds. */
  every: number;
}

export const DEFAULT_POMODORO: PomodoroSettings = { work: 25, short: 5, long: 15, every: 4 };

/** Generous on both ends: the point is to bound a cram, not to police the clock. */
export const POMODORO_LIMITS = { work: [5, 90], short: [1, 30], long: [5, 60], every: [2, 8] };

function isSettings(value: unknown): value is PomodoroSettings {
  if (!value || typeof value !== 'object') return false;
  const settings = value as Record<string, unknown>;
  return (Object.keys(POMODORO_LIMITS) as (keyof PomodoroSettings)[]).every((key) => {
    const [min, max] = POMODORO_LIMITS[key];
    const candidate = settings[key];
    return (
      typeof candidate === 'number' &&
      Number.isInteger(candidate) &&
      candidate >= min &&
      candidate <= max
    );
  });
}

/**
 * What the timer is doing, persisted so quitting mid-round is not the same as losing it.
 *
 * `endsAt` is a wall-clock instant rather than a countdown: an interval that misses ticks —
 * a sleeping machine, a backgrounded window, a busy main thread — would otherwise drift, and
 * a study timer that quietly runs long is worse than none.
 */
interface Runtime {
  phase: Phase;
  /** Completed work rounds, used to place the long break. */
  round: number;
  /** Epoch ms when the current phase ends, or `null` while paused. */
  endsAt: number | null;
  /** Milliseconds left, meaningful while paused. */
  left: number;
}

function isRuntime(value: unknown): value is Runtime {
  if (!value || typeof value !== 'object') return false;
  const { phase, round, endsAt, left } = value as Partial<Runtime>;
  return (
    (phase === 'work' || phase === 'short' || phase === 'long') &&
    typeof round === 'number' &&
    (endsAt === null || typeof endsAt === 'number') &&
    typeof left === 'number'
  );
}

const minutes = (value: number) => value * 60_000;

function freshRuntime(settings: PomodoroSettings): Runtime {
  return { phase: 'work', round: 0, endsAt: null, left: minutes(settings.work) };
}

function durationOf(phase: Phase, settings: PomodoroSettings): number {
  return minutes(
    phase === 'work' ? settings.work : phase === 'short' ? settings.short : settings.long,
  );
}

/** What follows the phase that just ended. */
export function advance(runtime: Runtime, settings: PomodoroSettings): Runtime {
  if (runtime.phase !== 'work') {
    return { phase: 'work', round: runtime.round, endsAt: null, left: minutes(settings.work) };
  }
  const round = runtime.round + 1;
  const phase: Phase = round % settings.every === 0 ? 'long' : 'short';
  return { phase, round, endsAt: null, left: durationOf(phase, settings) };
}

export function formatClock(ms: number): string {
  const total = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
}

/**
 * Work/rest rounds, shared by the whole app from the header.
 *
 * Deliberately advisory: reaching zero announces itself and stops, it never blocks or
 * navigates. The student decides whether to take the break; the timer's job is to make sure
 * they know three hours went by.
 */
export function usePomodoro(onPhaseEnd: (next: Phase) => void) {
  const [settings, setSettings] = usePreference<PomodoroSettings>(
    'grind:pomodoro-settings',
    DEFAULT_POMODORO,
    isSettings,
  );
  const [runtime, setRuntime] = usePreference<Runtime>(
    'grind:pomodoro',
    freshRuntime(settings),
    isRuntime,
  );
  // Re-rendered every second while running; the value itself lives in `runtime`.
  const [, tick] = useState(0);

  // Kept in a ref so the interval below never has to be torn down and rebuilt to see a new
  // handler, which would restart the second it fires on.
  const announce = useRef(onPhaseEnd);
  announce.current = onPhaseEnd;

  const running = runtime.endsAt !== null;
  const left = runtime.endsAt === null ? runtime.left : Math.max(0, runtime.endsAt - Date.now());

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => tick((value) => value + 1), 500);
    return () => window.clearInterval(id);
  }, [running]);

  useEffect(() => {
    if (!running || left > 0) return;
    const next = advance(runtime, settings);
    setRuntime(next);
    announce.current(next.phase);
  }, [running, left, runtime, settings, setRuntime]);

  const start = useCallback(() => {
    setRuntime({ ...runtime, endsAt: Date.now() + runtime.left });
  }, [runtime, setRuntime]);

  const pause = useCallback(() => {
    setRuntime({ ...runtime, endsAt: null, left });
  }, [runtime, left, setRuntime]);

  const reset = useCallback(() => {
    setRuntime({ ...runtime, endsAt: null, left: durationOf(runtime.phase, settings) });
  }, [runtime, settings, setRuntime]);

  const skip = useCallback(() => {
    setRuntime(advance(runtime, settings));
  }, [runtime, settings, setRuntime]);

  /**
   * Changing a duration re-times the current phase only when it is paused at full length —
   * otherwise a longer "work" would silently extend a round already half spent.
   */
  const applySettings = useCallback(
    (next: PomodoroSettings) => {
      setSettings(next);
      if (runtime.endsAt === null && runtime.left === durationOf(runtime.phase, settings)) {
        setRuntime({ ...runtime, left: durationOf(runtime.phase, next) });
      }
    },
    [runtime, settings, setRuntime, setSettings],
  );

  return {
    phase: runtime.phase,
    round: runtime.round,
    running,
    left,
    total: durationOf(runtime.phase, settings),
    start,
    pause,
    reset,
    skip,
    settings,
    setSettings: applySettings,
  };
}
