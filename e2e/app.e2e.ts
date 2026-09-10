import { expect, openApp, plan, startSession, TOPICS, test } from './fixtures';

test('hand-picked topics replace the scheduler', async ({ page, backend }) => {
  backend.handlers.plan_study_session = () => plan(true);

  await openApp(page);
  await page.getByRole('button', { name: 'Open', exact: true }).click();
  await page.getByRole('button', { name: 'Pick topics' }).click();
  await page.getByRole('checkbox', { name: /Друга тема/ }).click();
  await expect(page.getByText('1 of 10 chosen')).toBeVisible();
  await page.getByRole('button', { name: 'Start session (1)' }).click();

  await expect
    .poll(() => backend.argsOf('plan_study_session'))
    .toEqual([
      {
        subjectId: 'demo',
        size: 1,
        mode: 'sprint',
        selection: 'scheduled',
        topicIds: ['demo/1.2'],
      },
    ]);
});

test.describe('first run', () => {
  test.use({ firstRun: true });

  test('nothing is read before setup, and the guide shows once after it', async ({
    page,
    backend,
  }) => {
    await openApp(page);
    await page.getByRole('button', { name: 'Use this', exact: true }).click();
    expect(backend.argsOf('list_subjects')).toEqual([]);

    await page.getByRole('button', { name: 'Skip', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Subjects' })).toBeVisible();
  });
});

test('a session waits for its questions and grades exactly what was entered', async ({
  page,
  backend,
}) => {
  let release = () => {};
  const questionsLanded = new Promise<void>((resolve) => {
    release = resolve;
  });
  backend.handlers.prepare_session_questions = async () => {
    await questionsLanded;
    return plan(true);
  };

  await startSession(page);
  await expect(page.getByRole('heading', { name: 'Перша тема' })).toBeVisible();
  await page.getByRole('button', { name: 'Next topic' }).click();
  await page.getByRole('button', { name: 'To the questions' }).click();
  await expect(page.getByRole('button', { name: 'Preparing questions…' })).toBeDisabled();

  release();
  await page.getByRole('textbox').first().fill('Моя відповідь');
  await page.getByRole('button', { name: 'To the quiz' }).click();
  await page.getByRole('button', { name: 'Правильно 1' }).click();
  await page.getByRole('button', { name: 'Finish session' }).click();

  await expect(page.getByText('Session score')).toBeVisible();
  expect(backend.argsOf('finish_study_session')).toEqual([
    {
      subjectId: 'demo',
      sessionId: 'session-1',
      openAnswers: { 'demo/1.1': 'Моя відповідь' },
      quizSelections: { q1: [0] },
    },
  ]);
});

test('a written answer survives leaving and relaunching mid-session', async ({ page, backend }) => {
  backend.handlers.plan_study_session = () => plan(true);

  await startSession(page);
  await page.getByRole('button', { name: 'Next topic' }).click();
  await page.getByRole('button', { name: 'To the questions' }).click();
  await page.getByRole('textbox').first().fill('Недописана відповідь');

  backend.handlers.unfinished_sessions = () => [
    {
      id: 'session-1',
      subject: 'demo',
      created_at: '2026-09-10T10:00:00Z',
      topic_titles: TOPICS.map(({ title }) => title),
    },
  ];
  await page.getByRole('button', { name: 'Exit', exact: true }).click();
  await page.reload();

  await page.getByRole('button', { name: 'Open', exact: true }).click();
  await page.getByRole('button', { name: 'Resume', exact: true }).click();
  await page.getByRole('button', { name: 'Next topic' }).click();
  await page.getByRole('button', { name: 'To the questions' }).click();
  await expect(page.getByRole('textbox').first()).toHaveValue('Недописана відповідь');
  expect(backend.argsOf('resume_study_session')).toEqual([
    { subjectId: 'demo', sessionId: 'session-1' },
  ]);
});

test('a failed question generation can be retried without leaving the notes', async ({
  page,
  backend,
}) => {
  let failing = true;
  backend.handlers.prepare_session_questions = () => {
    if (failing) throw new Error('agy failed twice');
    return plan(true);
  };

  await startSession(page);
  await expect(page.getByText('The questions could not be generated')).toBeVisible();
  await page.getByRole('button', { name: 'Next topic' }).click();

  failing = false;
  await page.getByRole('button', { name: 'Try again' }).click();
  await page.getByRole('button', { name: 'To the questions' }).click();
  await expect(page.getByRole('textbox')).toHaveCount(2);
});
