import { test as base, expect, type Page } from '@playwright/test';

import type {
  KnowledgeNote,
  Question,
  SessionPlan,
  SessionResult,
  SubjectDetail,
  SubjectOverview,
  Topic,
  VaultInfo,
} from '../src/lib/types';

type Handler = (args: Record<string, unknown>) => unknown;

/** The Tauri command layer as the page sees it. Tests swap handlers before acting. */
export interface Backend {
  handlers: Record<string, Handler>;
  argsOf: (command: string) => Record<string, unknown>[];
}

const topic = (index: number, title: string): Topic => ({
  id: `demo/1.${index}`,
  subject: 'demo',
  section: 1,
  section_title: 'Основи',
  index,
  title,
});

export const TOPICS = [topic(1, 'Перша тема'), topic(2, 'Друга тема')];

const vault = (configured: boolean): VaultInfo => ({
  root: '/vault',
  configured,
  readable: configured,
  subject_count: 1,
  error: null,
  agy_binary: '/usr/local/bin/agy',
});

const note = ({ id, title, section_title }: Topic): KnowledgeNote => ({
  topic_id: id,
  subject: 'demo',
  title,
  section_title,
  model: 'smart',
  generated_at: '2026-09-01T00:00:00Z',
  body: `Конспект: ${title}.`,
  path: `/vault/knowledge/${id}.md`,
});

const question = ({ id, title }: Topic, index: number): Question => ({
  id: `q${index + 1}`,
  topic_id: id,
  type: 'single',
  question: `Питання про ${title}`,
  options: [`Правильно ${index + 1}`, `Хибно ${index + 1}`],
  correct: [0],
  explanation: '',
  difficulty: 'medium',
});

export const plan = (withQuestions: boolean): SessionPlan => ({
  id: 'session-1',
  subject: 'demo',
  created_at: '2026-09-10T10:00:00Z',
  mode: 'full',
  topics: TOPICS.map((entry) => ({
    topic: entry,
    stage: 'new',
    level: 0,
    last_score: null,
    is_review: false,
  })),
  notes: TOPICS.map(note),
  reading: TOPICS.map((entry) => note(entry).body),
  open_questions: withQuestions
    ? TOPICS.map(({ id, title }) => ({ topic_id: id, question: title, expected_points: ['суть'] }))
    : [],
  quiz: withQuestions ? TOPICS.map(question) : [],
});

const overview: SubjectOverview = {
  id: 'demo',
  title: 'Демо-предмет',
  topic_count: 2,
  knowledge_count: 2,
  quiz_count: 0,
  accuracy_percent: 0,
};

const detail: SubjectDetail = {
  subject: {
    id: 'demo',
    title: 'Демо-предмет',
    sections: [{ index: 1, title: 'Основи', topics: TOPICS }],
  },
  knowledge_topic_ids: TOPICS.map(({ id }) => id),
  quizzes: [],
  stats: {
    subject: 'demo',
    topics_total: 2,
    topics_with_knowledge: 2,
    topics_practised: 0,
    questions_answered: 0,
    questions_correct: 0,
    accuracy_percent: 0,
    weakest: [],
  },
  study: {
    subject: 'demo',
    available: 2,
    new: 2,
    learning: 0,
    review: 0,
    mastered: 0,
    due_now: 0,
    studied_today: 0,
    progress_percent: 0,
  },
  topic_study: {},
};

const result: SessionResult = {
  session_id: 'session-1',
  subject: 'demo',
  gradings: [],
  answers: [],
  topics: TOPICS.map(({ id, title }) => ({
    topic_id: id,
    title,
    score: 85,
    open_score: 80,
    quiz_correct: 1,
    quiz_total: 1,
    level: 1,
    stage: 'learning',
    due_at: null,
    skipped: false,
  })),
  overall_score: 85,
};

const defaultHandlers = (firstRun: boolean): Record<string, Handler> => ({
  vault_info: () => vault(!firstRun),
  use_default_vault: () => vault(true),
  list_subjects: () => [overview],
  get_subject: () => detail,
  unfinished_sessions: () => [],
  plan_study_session: () => plan(false),
  prepare_session_questions: () => plan(true),
  resume_study_session: () => plan(true),
  mark_topic_read: () => null,
  finish_study_session: () => result,
});

export const test = base.extend<{ firstRun: boolean; backend: Backend }>({
  firstRun: [false, { option: true }],
  backend: [
    async ({ page, firstRun }, use) => {
      const handlers = defaultHandlers(firstRun);
      const calls: { command: string; args: Record<string, unknown> }[] = [];
      const unhandled: string[] = [];

      await page.exposeFunction('fakeBackend', (command: string, args: Record<string, unknown>) => {
        calls.push({ command, args });
        const handler = handlers[command];
        if (handler) return handler(args);
        unhandled.push(command);
        throw new Error(`the fake backend has no handler for ${command}`);
      });
      await page.addInitScript((isFirstRun) => {
        localStorage.setItem('grind:locale', '"en"');
        if (!isFirstRun) localStorage.setItem('grind:guide-seen', 'true');
      }, firstRun);

      await use({
        handlers,
        argsOf: (command) =>
          calls.filter((call) => call.command === command).map((call) => call.args),
      });
      expect(unhandled, 'commands the page invoked that the fake does not implement').toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };

export async function openApp(page: Page) {
  await page.goto('/e2e/index.html');
}

export async function startSession(page: Page) {
  await openApp(page);
  await page.getByRole('button', { name: 'Open', exact: true }).click();
  await page.getByRole('button', { name: 'Start session' }).click();
}
