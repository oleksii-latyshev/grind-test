/**
 * Mirrors the serde types in `src-tauri/src/`. Keep both sides in sync by hand —
 * Rust owns the vault, this file only describes what comes across `invoke()`.
 */

export type Difficulty = "easy" | "medium" | "hard" | "mixed";
export type QuestionKind = "single" | "multi";

export interface Topic {
  id: string;
  subject: string;
  section: number;
  section_title: string;
  index: number;
  title: string;
}

export interface Section {
  index: number;
  title: string;
  topics: Topic[];
}

export interface Subject {
  id: string;
  title: string;
  sections: Section[];
}

export interface SubjectOverview {
  id: string;
  title: string;
  topic_count: number;
  knowledge_count: number;
  quiz_count: number;
  accuracy_percent: number;
}

export interface WeakTopic {
  topic_id: string;
  title: string;
  seen: number;
  correct: number;
  accuracy_percent: number;
}

export interface SubjectStats {
  subject: string;
  topics_total: number;
  topics_with_knowledge: number;
  topics_practised: number;
  questions_answered: number;
  questions_correct: number;
  accuracy_percent: number;
  weakest: WeakTopic[];
}

export interface QuizSummary {
  id: string;
  subject: string;
  title: string;
  created_at: string;
  question_count: number;
  difficulty: Difficulty;
}

export interface SubjectDetail {
  subject: Subject;
  knowledge_topic_ids: string[];
  quizzes: QuizSummary[];
  stats: SubjectStats;
}

export interface KnowledgeNote {
  topic_id: string;
  subject: string;
  title: string;
  section_title: string;
  model: string;
  generated_at: string;
  body: string;
  path: string;
}

export interface Question {
  id: string;
  topic_id: string;
  type: QuestionKind;
  question: string;
  options: string[];
  /** 0-based indices into `options`. */
  correct: number[];
  explanation: string;
  difficulty: Difficulty;
}

export interface Quiz {
  id: string;
  subject: string;
  title: string;
  created_at: string;
  model: string;
  difficulty: Difficulty;
  topic_ids: string[];
  questions: Question[];
}

export interface AnswerRecord {
  question_id: string;
  topic_id: string;
  selected: number[];
  correct: boolean;
  hint_used: boolean;
}

export interface Attempt {
  id: string;
  quiz_id: string;
  subject: string;
  quiz_title: string;
  finished_at: string;
  answers: AnswerRecord[];
}

export interface AttemptResult {
  attempt: Attempt;
  total: number;
  correct: number;
  score_percent: number;
}

export interface Hint {
  hint: string;
  related_concepts: string[];
}

export interface KnowledgeBatchReport {
  generated: number;
  skipped: number;
  failed: number;
  total_tokens: number;
  errors: string[];
}

export interface VaultInfo {
  root: string;
  exists: boolean;
  agy_binary: string | null;
}

export interface QuizRequest {
  subject: string;
  topic_ids?: string[] | null;
  question_count: number;
  difficulty: Difficulty;
}

export type GenerationEvent =
  | { kind: "started"; subject: string; total: number; skipped: number }
  | { kind: "topic_started"; topic_id: string; title: string }
  | { kind: "topic_done"; topic_id: string; title: string; done: number; total: number }
  | { kind: "topic_failed"; topic_id: string; title: string; error: string }
  | { kind: "finished"; generated: number; failed: number; total_tokens: number };
