import type { Stage } from "./types";

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
