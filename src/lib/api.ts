import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  Attempt,
  AttemptResult,
  GenerationEvent,
  Hint,
  KnowledgeBatchReport,
  KnowledgeNote,
  Quiz,
  QuizRequest,
  QuizSummary,
  SubjectDetail,
  SubjectOverview,
  VaultInfo,
} from "./types";

const GENERATION_EVENT = "generation://progress";

export const api = {
  vaultInfo: () => invoke<VaultInfo>("vault_info"),

  listSubjects: () => invoke<SubjectOverview[]>("list_subjects"),

  getSubject: (subjectId: string) => invoke<SubjectDetail>("get_subject", { subjectId }),

  getKnowledge: (subjectId: string, topicId: string) =>
    invoke<KnowledgeNote | null>("get_knowledge", { subjectId, topicId }),

  /** Long-running: subscribe with `onGenerationProgress` before awaiting this. */
  generateKnowledge: (options: {
    subjectId: string;
    topicIds?: string[] | null;
    force?: boolean;
    concurrency?: number;
  }) =>
    invoke<KnowledgeBatchReport>("generate_knowledge", {
      subjectId: options.subjectId,
      topicIds: options.topicIds ?? null,
      force: options.force ?? false,
      concurrency: options.concurrency ?? 4,
    }),

  generateQuiz: (request: QuizRequest) => invoke<Quiz>("generate_quiz", { request }),

  listQuizzes: (subjectId: string) => invoke<QuizSummary[]>("list_quizzes", { subjectId }),

  getQuiz: (subjectId: string, quizId: string) => invoke<Quiz>("get_quiz", { subjectId, quizId }),

  submitQuiz: (options: {
    subjectId: string;
    quizId: string;
    selections: Record<string, number[]>;
    hintsUsed: string[];
  }) =>
    invoke<AttemptResult>("submit_quiz", {
      subjectId: options.subjectId,
      quizId: options.quizId,
      selections: options.selections,
      hintsUsed: options.hintsUsed,
    }),

  getHint: (subjectId: string, quizId: string, questionId: string) =>
    invoke<Hint>("get_hint", { subjectId, quizId, questionId }),

  listAttempts: (subjectId?: string) =>
    invoke<Attempt[]>("list_attempts", { subjectId: subjectId ?? null }),
};

export function onGenerationProgress(
  handler: (event: GenerationEvent) => void,
): Promise<UnlistenFn> {
  return listen<GenerationEvent>(GENERATION_EVENT, (event) => handler(event.payload));
}

/** Tauri rejects with a plain string; everything else may not be an Error. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
