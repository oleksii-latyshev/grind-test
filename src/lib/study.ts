import type { Stage, TopicStudy } from "./types";

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
