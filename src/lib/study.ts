import type { MessageId, Translate } from '@/i18n';

import type { SessionMode, Stage, TopicStudy } from './types';

/**
 * Message id and colour role for each rung of the ladder, shared by every view.
 *
 * The id rather than the text: the label is translated, the colour is not.
 */
export const STAGES: Record<Stage, { label: MessageId; className: string }> = {
  new: { label: 'stage.new', className: 'text-muted-foreground' },
  reading: { label: 'stage.reading', className: 'text-chart-2' },
  learning: { label: 'stage.learning', className: 'text-chart-2' },
  review: { label: 'stage.review', className: 'text-chart-3' },
  mastered: { label: 'stage.mastered', className: 'text-primary' },
};

/** Mirrors REVIEW_INTERVALS_DAYS in `src-tauri/src/vault/study.rs`. */
export const MAX_LEVEL = 6;

/** Mirrors MAX_SESSION_TOPICS in `src-tauri/src/generate.rs`. */
export const MAX_SESSION_TOPICS = 6;

/** The ladder thresholds live in one place rather than in each view that renders a badge. */
export function stageOf(entry: TopicStudy | undefined): Stage {
  if (!entry) return 'new';
  if (entry.level === 0) return entry.read_count > 0 ? 'reading' : 'new';
  if (entry.level <= 2) return 'learning';
  if (entry.level <= 4) return 'review';
  return 'mastered';
}

export function formatDue(t: Translate, dueAt: string | null): string {
  if (!dueAt) return t('due.none');
  const due = new Date(dueAt);
  const days = Math.round((due.getTime() - Date.now()) / 86_400_000);
  if (days <= 0) return t('due.today');
  if (days === 1) return t('due.tomorrow');
  return t('due.days', { days });
}

export function scoreTone(score: number): string {
  if (score >= 80) return 'text-primary';
  if (score >= 60) return 'text-chart-3';
  return 'text-destructive';
}

/**
 * Minutes a topic takes end to end — reading, answering, quiz, reading the feedback.
 *
 * `full` is measured: roughly 20 minutes per topic in practice. The others are estimates
 * built from the parts that shrink — `balanced` keeps the reading and replaces several
 * paragraphs of writing with a bulleted recall; `sprint` also cuts the note to its digest,
 * about a third of the text.
 */
export const MINUTES_PER_TOPIC: Record<SessionMode, number> = {
  full: 20,
  balanced: 13,
  sprint: 8,
};

export const SESSION_SIZES: Record<SessionMode, number[]> = {
  full: [2, 3, 5],
  balanced: [3, 5, 8],
  sprint: [5, 8, 10],
};

export const MAX_TOPICS: Record<SessionMode, number> = {
  full: 6,
  balanced: 8,
  sprint: 10,
};

export function formatHours(t: Translate, minutes: number): string {
  const hours = minutes / 60;
  if (hours < 1) return t('time.minutes', { value: Math.round(minutes) });
  return t('time.hours', { value: hours < 10 ? hours.toFixed(1) : Math.round(hours) });
}
