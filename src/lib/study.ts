import type { SessionMode, Stage, TopicStudy } from "./types";

/** Ukrainian labels and colour roles for the study ladder, shared by every view. */
export const STAGES: Record<Stage, { label: string; className: string }> = {
  new: { label: "Не почато", className: "text-muted-foreground" },
  reading: { label: "Прочитано", className: "text-chart-2" },
  learning: { label: "Вивчається", className: "text-chart-2" },
  review: { label: "Повторення", className: "text-chart-3" },
  mastered: { label: "Засвоєно", className: "text-primary" },
};

/** Mirrors REVIEW_INTERVALS_DAYS in `src-tauri/src/vault/study.rs`. */
export const MAX_LEVEL = 6;

/** Mirrors MAX_SESSION_TOPICS in `src-tauri/src/generate.rs`. */
export const MAX_SESSION_TOPICS = 6;

/** The ladder thresholds live in one place rather than in each view that renders a badge. */
export function stageOf(entry: TopicStudy | undefined): Stage {
  if (!entry) return "new";
  if (entry.level === 0) return entry.read_count > 0 ? "reading" : "new";
  if (entry.level <= 2) return "learning";
  if (entry.level <= 4) return "review";
  return "mastered";
}

export function formatDue(dueAt: string | null): string {
  if (!dueAt) return "—";
  const due = new Date(dueAt);
  const days = Math.round((due.getTime() - Date.now()) / 86_400_000);
  if (days <= 0) return "сьогодні";
  if (days === 1) return "завтра";
  return `через ${days} дн.`;
}

export function scoreTone(score: number): string {
  if (score >= 80) return "text-primary";
  if (score >= 60) return "text-chart-3";
  return "text-destructive";
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

export function formatHours(minutes: number): string {
  const hours = minutes / 60;
  if (hours < 1) return `${Math.round(minutes)} хв`;
  return `${hours < 10 ? hours.toFixed(1) : Math.round(hours)} год`;
}
