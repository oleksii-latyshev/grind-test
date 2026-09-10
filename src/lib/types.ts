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
  study: StudyOverview;
  /** Per-topic study state; a missing key means the topic was never studied. */
  topic_study: Record<string, TopicStudy>;
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
  /** Setup is done and the student has said where the vault is. */
  configured: boolean;
  /** The app can list the syllabus directory right now. */
  readable: boolean;
  subject_count: number;
  /** Why it is not readable, phrased for the user. */
  error: string | null;
  agy_binary: string | null;
}

/** Which model id each tier resolves to; mirrors `agy::ModelSettings`. */
export interface ModelSettings {
  smart: string;
  fast: string;
}

/** One entry from `agy models`. */
export interface ModelInfo {
  id: string;
  label: string;
}

export interface ModelChoice {
  models: ModelSettings;
  /** What the app ships with, for "back to defaults". */
  defaults: ModelSettings;
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

/* ------------------------------------------------------------------ study */

export type Stage = "new" | "reading" | "learning" | "review" | "mastered";

export interface TopicStudy {
  topic_id: string;
  /** 0 = never studied; 1..6 index into the review ladder. */
  level: number;
  read_count: number;
  sessions: number;
  last_studied: string | null;
  due_at: string | null;
  last_score: number | null;
}

export interface StudyOverview {
  subject: string;
  available: number;
  new: number;
  learning: number;
  review: number;
  mastered: number;
  due_now: number;
  studied_today: number;
  progress_percent: number;
}

export interface PlannedTopic {
  topic: Topic;
  stage: Stage;
  level: number;
  last_score: number | null;
  is_review: boolean;
}

export interface OpenQuestion {
  topic_id: string;
  question: string;
  /** The grading rubric: what a complete answer has to contain. */
  expected_points: string[];
}

/** How a session trades depth for coverage. */
export type SessionMode = "full" | "balanced" | "sprint";

/** How topics are chosen when they are not listed explicitly. */
export type Selection = "scheduled" | "spread";

export interface SessionPlan {
  id: string;
  subject: string;
  created_at: string;
  mode: SessionMode;
  topics: PlannedTopic[];
  notes: KnowledgeNote[];
  /** What to show per topic, aligned with `topics`: full note, or digest in a sprint. */
  reading: string[];
  /** Empty until `prepare_session_questions` has run — see `sessionReady`. */
  open_questions: OpenQuestion[];
  /** Empty until `prepare_session_questions` has run — see `sessionReady`. */
  quiz: Question[];
}

/**
 * A session is ready once its questions exist. The generator refuses to return an empty
 * quiz, so an empty one can only mean the call has not landed yet — no extra flag needed,
 * and sessions stored before the split still read as ready.
 */
export function sessionReady(plan: SessionPlan): boolean {
  return plan.quiz.length > 0;
}

export interface OpenGrading {
  topic_id: string;
  score: number;
  verdict: string;
  covered: string[];
  missed: string[];
  correction: string;
}

export interface TopicOutcome {
  topic_id: string;
  title: string;
  score: number;
  open_score: number | null;
  quiz_correct: number;
  quiz_total: number;
  level: number;
  stage: Stage;
  due_at: string | null;
  /** Both halves left blank: read, not scored, schedule untouched. */
  skipped: boolean;
}

export interface SessionSummary {
  id: string;
  subject: string;
  created_at: string;
  topic_titles: string[];
}

export interface SessionResult {
  session_id: string;
  subject: string;
  gradings: OpenGrading[];
  answers: AnswerRecord[];
  topics: TopicOutcome[];
  overall_score: number;
}
