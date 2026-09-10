import {
  ArrowRight,
  Check,
  ChevronLeft,
  Loader2,
  PenLine,
  Repeat,
  RotateCw,
  Sparkles,
  TriangleAlert,
  X,
  Zap,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { NoteReader } from '@/components/NoteReader';
import { ReadingSizeControl } from '@/components/ReadingSizeControl';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Textarea } from '@/components/ui/textarea';
import type { MessageId, Translate } from '@/i18n';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import { clearDraft, readDraft, writeDraft } from '@/lib/drafts';
import { readSessionSettings } from '@/lib/prefs';
import { formatDue, STAGES, scoreTone } from '@/lib/study';
import type { SessionPlan, SessionResult } from '@/lib/types';
import { sessionReady } from '@/lib/types';
import { cn } from '@/lib/utils';

type Step = 'read' | 'open' | 'quiz' | 'done';

const STEP_LABELS: Record<Exclude<Step, 'done'>, MessageId> = {
  read: 'session.step.read',
  open: 'session.step.open',
  quiz: 'session.step.quiz',
};

interface Props {
  plan: SessionPlan;
  onExit: () => void;
  onFinished: () => void;
  /** Start straight into the next session, without going back through the menu. */
  onContinue: (plan: SessionPlan) => void;
}

export function StudySession({ plan: initial, onExit, onFinished, onContinue }: Props) {
  const t = useT();
  const [step, setStep] = useState<Step>('read');
  const [readIndex, setReadIndex] = useState(0);
  // The plan arrives with notes but usually without questions; the generation call runs
  // behind the reading and replaces it here when it lands.
  const [plan, setPlan] = useState(initial);
  const [prepareError, setPrepareError] = useState<string | null>(null);
  // Bumped by the retry button; re-runs the effect below without remounting the session.
  const [retry, setRetry] = useState(0);
  // Set when the last forward button was pressed before the questions were ready: reading is
  // finished, so the only thing left to do is wait, and the step advances by itself.
  const [waiting, setWaiting] = useState(false);
  // Restored from the local draft, so re-entering a session brings the typing back.
  const [openAnswers, setOpenAnswers] = useState<Record<string, string>>(() =>
    readDraft(initial.id),
  );
  const [quizSelections, setQuizSelections] = useState<Record<string, number[]>>({});
  const [result, setResult] = useState<SessionResult | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const ready = sessionReady(plan);

  useEffect(() => {
    writeDraft(plan.id, openAnswers);
  }, [plan.id, openAnswers]);

  // One attempt per session, started as the first note opens. The command is idempotent, so
  // a resumed session that already has its questions simply gets them back.
  // biome-ignore lint/correctness/useExhaustiveDependencies: `retry` exists only to re-run this effect.
  useEffect(() => {
    if (sessionReady(initial)) return;
    let cancelled = false;
    setPrepareError(null);
    api
      .prepareSessionQuestions(initial.subject, initial.id)
      .then((prepared) => {
        if (!cancelled) setPlan(prepared);
      })
      .catch((error) => {
        if (!cancelled) setPrepareError(errorMessage(error));
      });
    return () => {
      cancelled = true;
    };
  }, [initial, retry]);

  // The student pressed on past the last note before the call landed.
  useEffect(() => {
    if (waiting && ready) {
      setWaiting(false);
      setStep('open');
      window.scrollTo({ top: 0 });
    }
  }, [waiting, ready]);

  function advanceReading() {
    const topic = plan.topics[readIndex];
    // Fire and forget: a failed bookkeeping write must not block the session.
    api.markTopicRead(topic.topic.id).catch(() => {});
    if (readIndex + 1 < plan.topics.length) {
      setReadIndex((value) => value + 1);
      window.scrollTo({ top: 0 });
      return;
    }
    if (!ready) {
      // Nothing to show yet; the effect above moves on as soon as there is.
      setWaiting(true);
      return;
    }
    setStep('open');
    window.scrollTo({ top: 0 });
  }

  function chooseOption(questionId: string, option: number, multi: boolean) {
    setQuizSelections((current) => {
      const previous = current[questionId] ?? [];
      if (!multi) return { ...current, [questionId]: [option] };
      const next = previous.includes(option)
        ? previous.filter((value) => value !== option)
        : [...previous, option];
      return { ...current, [questionId]: next };
    });
  }

  async function submit() {
    setSubmitting(true);
    try {
      const outcome = await api.finishStudySession({
        subjectId: plan.subject,
        sessionId: plan.id,
        openAnswers,
        quizSelections,
      });
      setResult(outcome);
      clearDraft(plan.id);
      setStep('done');
      window.scrollTo({ top: 0 });
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  }

  if (step === 'done' && result) {
    return (
      <SessionSummary
        plan={plan}
        result={result}
        openAnswers={openAnswers}
        onDone={onFinished}
        onContinue={onContinue}
        t={t}
      />
    );
  }

  const answeredQuiz = Object.values(quizSelections).filter((s) => s.length > 0).length;
  // Two independent choices: a sprint shortens the reading, and everything but the full
  // mode shortens the answer.
  const digest = plan.mode === 'sprint';
  const listAnswer = plan.mode !== 'full';

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-8">
      <header className="sticky top-12 z-10 -mx-8 space-y-3 border-b border-border bg-background/95 px-8 py-3 backdrop-blur">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            {(['read', 'open', 'quiz'] as const).map((value, index) => (
              <span key={value} className="flex items-center gap-2">
                {index > 0 ? <span className="text-border">/</span> : null}
                <span className={cn(step === value && 'font-medium text-foreground')}>
                  {t(STEP_LABELS[value])}
                </span>
              </span>
            ))}
          </div>
          <div className="flex items-center gap-2">
            {step === 'read' ? <ReadingSizeControl /> : null}
            <Button variant="ghost" size="sm" onClick={onExit}>
              <X className="size-4" />
              {t('common.exit')}
            </Button>
          </div>
        </div>
        <Progress
          value={
            step === 'read'
              ? ((readIndex + 1) / plan.topics.length) * 60
              : step === 'open'
                ? 75
                : 90
          }
        />
      </header>

      {step === 'read' ? (
        <>
          <NoteReader
            note={{
              ...plan.notes[readIndex],
              // A sprint reads the digest; older sessions have no `reading` array.
              body: plan.reading[readIndex] ?? plan.notes[readIndex].body,
            }}
            eyebrow={
              <>
                <Badge variant="outline" className="font-mono text-[0.7rem]">
                  {plan.topics[readIndex].topic.id}
                </Badge>
                <span className="text-xs text-muted-foreground">
                  {t('session.topicOf', { index: readIndex + 1, total: plan.topics.length })}
                </span>
                {digest ? (
                  <Badge variant="secondary" className="gap-1">
                    <Zap className="size-3" />
                    {t('session.digest')}
                  </Badge>
                ) : null}
                {plan.topics[readIndex].is_review ? (
                  <Badge variant="secondary" className="gap-1">
                    <Repeat className="size-3" />
                    {t('session.review')}
                  </Badge>
                ) : null}
              </>
            }
          />
          <div className="mx-auto flex w-full max-w-3xl items-center justify-between border-t border-border pt-6">
            <Button
              variant="outline"
              onClick={() => setReadIndex((value) => value - 1)}
              disabled={readIndex === 0}
            >
              <ChevronLeft className="size-4" />
              {t('session.prev')}
            </Button>
            {prepareError && readIndex + 1 === plan.topics.length ? (
              <Button
                variant="outline"
                onClick={() => {
                  setPrepareError(null);
                  setRetry((value) => value + 1);
                }}
              >
                <RotateCw className="size-4" />
                {t('common.retry')}
              </Button>
            ) : (
              <Button onClick={advanceReading} disabled={waiting}>
                {waiting ? <Loader2 className="size-4 animate-spin" /> : null}
                {readIndex + 1 < plan.topics.length
                  ? t('session.nextTopic')
                  : waiting
                    ? t('session.preparing')
                    : t('session.toQuestions')}
                {waiting ? null : <ArrowRight className="size-4" />}
              </Button>
            )}
          </div>

          {prepareError ? (
            <p className="mx-auto flex w-full max-w-3xl items-start gap-2 text-sm text-destructive">
              <TriangleAlert className="mt-0.5 size-4 shrink-0" />
              <span>{t('session.prepareFailed', { error: prepareError })}</span>
            </p>
          ) : null}
        </>
      ) : step === 'open' ? (
        <section className="space-y-5">
          <div className="space-y-1">
            <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
              <PenLine className="size-5 text-primary" />
              {listAnswer ? t('session.open.listTitle') : t('session.open.title')}
            </h2>
            <p className="text-sm text-muted-foreground">
              {listAnswer ? t('session.open.listBlurb') : t('session.open.blurb')}{' '}
              {t('session.open.afterwards')}
            </p>
          </div>

          {plan.open_questions.map((question, index) => (
            <Card key={question.topic_id}>
              <CardContent className="space-y-3 py-5">
                <Badge variant="outline" className="font-mono text-[0.7rem]">
                  {question.topic_id}
                </Badge>
                <p className="text-sm font-medium leading-snug">
                  {index + 1}. {question.question}
                </p>
                <Textarea
                  rows={listAnswer ? 5 : 8}
                  value={openAnswers[question.topic_id] ?? ''}
                  onChange={(event) => {
                    // Read the value now: React clears `currentTarget` once the handler
                    // returns, and a functional update runs later, during render — reading
                    // it in there throws and takes the whole tree down.
                    const value = event.target.value;
                    setOpenAnswers((current) => ({
                      ...current,
                      [question.topic_id]: value,
                    }));
                  }}
                  placeholder={
                    listAnswer ? t('session.open.listPlaceholder') : t('session.open.placeholder')
                  }
                />
              </CardContent>
            </Card>
          ))}

          <div className="flex justify-between border-t border-border pt-6">
            <Button variant="outline" onClick={() => setStep('read')}>
              <ChevronLeft className="size-4" />
              {t('session.toNotes')}
            </Button>
            <Button onClick={() => setStep('quiz')}>
              {t('session.toQuiz')}
              <ArrowRight className="size-4" />
            </Button>
          </div>
        </section>
      ) : (
        <section className="space-y-5">
          <div className="space-y-1">
            <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
              <Sparkles className="size-5 text-primary" />
              {t('session.quiz.title')}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t('session.quiz.answered', { answered: answeredQuiz, total: plan.quiz.length })}
            </p>
          </div>

          {plan.quiz.map((question, index) => {
            const chosen = quizSelections[question.id] ?? [];
            const multi = question.type === 'multi';
            return (
              <Card key={question.id}>
                <CardContent className="space-y-3 py-5">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline" className="font-mono text-[0.7rem]">
                      {question.topic_id}
                    </Badge>
                    {multi ? <Badge variant="secondary">{t('session.multi')}</Badge> : null}
                  </div>
                  <p className="text-sm font-medium leading-snug">
                    {index + 1}. {question.question}
                  </p>
                  <ul className="space-y-2">
                    {question.options.map((option, optionIndex) => {
                      const active = chosen.includes(optionIndex);
                      return (
                        // biome-ignore lint/suspicious/noArrayIndexKey: options are addressed by position.
                        <li key={optionIndex}>
                          <button
                            type="button"
                            onClick={() => chooseOption(question.id, optionIndex, multi)}
                            aria-pressed={active}
                            className={cn(
                              'flex w-full items-start gap-3 rounded-4xl border px-4 py-2.5 text-left text-sm transition-colors',
                              active
                                ? 'border-primary bg-primary/10'
                                : 'border-input hover:bg-muted/60',
                            )}
                          >
                            <span
                              className={cn(
                                'mt-px flex size-5 shrink-0 items-center justify-center border text-[0.7rem] font-medium',
                                multi ? 'rounded-md' : 'rounded-full',
                                active
                                  ? 'border-primary bg-primary text-primary-foreground'
                                  : 'border-input text-muted-foreground',
                              )}
                            >
                              {optionIndex + 1}
                            </span>
                            <span className="leading-snug">{option}</span>
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </CardContent>
              </Card>
            );
          })}

          <div className="flex justify-between border-t border-border pt-6">
            <Button variant="outline" onClick={() => setStep('open')}>
              <ChevronLeft className="size-4" />
              {t('session.backToOpen')}
            </Button>
            <Button onClick={submit} disabled={submitting}>
              {submitting ? <Loader2 className="size-4 animate-spin" /> : null}
              {t('session.finish')}
            </Button>
          </div>
        </section>
      )}
    </div>
  );
}

function SessionSummary({
  plan,
  result,
  openAnswers,
  onDone,
  onContinue,
  t,
}: {
  plan: SessionPlan;
  result: SessionResult;
  openAnswers: Record<string, string>;
  onDone: () => void;
  onContinue: (plan: SessionPlan) => void;
  t: Translate;
}) {
  const answersByQuestion = new Map(result.answers.map((answer) => [answer.question_id, answer]));
  const [starting, setStarting] = useState(false);

  // Read at click time, not at render: the same settings the study tab is showing.
  // A hand-picked list cannot be repeated — there is nothing to pick from here — so that
  // one case offers no shortcut rather than quietly changing how topics are chosen.
  const settings = readSessionSettings();

  async function next() {
    setStarting(true);
    try {
      onContinue(
        await api.planStudySession({
          subjectId: plan.subject,
          size: settings.size,
          mode: settings.pace,
          selection: settings.pick === 'spread' ? 'spread' : 'scheduled',
        }),
      );
      window.scrollTo({ top: 0 });
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-8">
      <Card>
        <CardContent className="space-y-4 py-6 text-center">
          <p className="text-sm text-muted-foreground">{t('session.result')}</p>
          {result.topics.every((outcome) => outcome.skipped) ? (
            <p className="text-5xl font-semibold text-muted-foreground">—</p>
          ) : (
            <>
              <p
                className={cn(
                  'text-5xl font-semibold tabular-nums',
                  scoreTone(result.overall_score),
                )}
              >
                {result.overall_score}%
              </p>
              <Progress value={result.overall_score} />
            </>
          )}
          <div className="flex flex-wrap items-center justify-center gap-3">
            <Button variant="outline" onClick={onDone}>
              {t('session.toSubject')}
            </Button>
            {settings.pick === 'manual' ? null : (
              <Button onClick={next} disabled={starting}>
                {starting ? <Loader2 className="size-4 animate-spin" /> : null}
                {t('session.continue')}
                <ArrowRight className="size-4" />
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      <section className="space-y-3">
        <h2 className="text-sm font-semibold tracking-tight">{t('session.topicsNext')}</h2>
        {result.topics.map((outcome) => (
          <Card key={outcome.topic_id}>
            <CardContent className="flex flex-wrap items-center justify-between gap-3 py-4">
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-snug">{outcome.title}</p>
                <p className="text-xs text-muted-foreground">
                  {t(STAGES[outcome.stage].label)} · {t('session.level', { level: outcome.level })}{' '}
                  · {t('session.nextReview', { due: formatDue(t, outcome.due_at) })}
                </p>
              </div>
              <div className="flex items-center gap-3 text-sm">
                {outcome.quiz_total > 0 ? (
                  <span className="text-muted-foreground">
                    {t('session.quizScore', {
                      correct: outcome.quiz_correct,
                      total: outcome.quiz_total,
                    })}
                  </span>
                ) : null}
                {outcome.skipped ? (
                  <span className="text-sm text-muted-foreground">{t('session.skipped')}</span>
                ) : (
                  <span
                    className={cn('text-lg font-semibold tabular-nums', scoreTone(outcome.score))}
                  >
                    {outcome.score}%
                  </span>
                )}
              </div>
            </CardContent>
          </Card>
        ))}
      </section>

      {result.gradings.length > 0 ? (
        <section className="space-y-4">
          <h2 className="text-sm font-semibold tracking-tight">{t('session.openReview')}</h2>
          {result.gradings.map((grading) => {
            const question = plan.open_questions.find((q) => q.topic_id === grading.topic_id);
            return (
              <Card key={grading.topic_id}>
                <CardContent className="space-y-4 py-5">
                  <div className="flex items-start justify-between gap-3">
                    <p className="flex-1 text-sm font-medium leading-snug">{question?.question}</p>
                    <span
                      className={cn('text-lg font-semibold tabular-nums', scoreTone(grading.score))}
                    >
                      {grading.score}%
                    </span>
                  </div>

                  <p className="text-sm leading-relaxed text-muted-foreground">{grading.verdict}</p>

                  {grading.covered.length > 0 ? (
                    <PointList icon="check" title={t('session.covered')} items={grading.covered} />
                  ) : null}
                  {grading.missed.length > 0 ? (
                    <PointList icon="cross" title={t('session.missed')} items={grading.missed} />
                  ) : null}

                  {grading.correction ? (
                    <div className="rounded-4xl border border-border bg-muted/50 p-4">
                      <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                        {t('session.correction')}
                      </p>
                      <p className="text-sm leading-relaxed">{grading.correction}</p>
                    </div>
                  ) : null}

                  {openAnswers[grading.topic_id]?.trim() ? (
                    <details className="text-sm">
                      <summary className="cursor-pointer text-xs uppercase tracking-wide text-muted-foreground">
                        {t('session.yourAnswer')}
                      </summary>
                      <p className="mt-2 whitespace-pre-wrap leading-relaxed text-muted-foreground">
                        {openAnswers[grading.topic_id]}
                      </p>
                    </details>
                  ) : null}
                </CardContent>
              </Card>
            );
          })}
        </section>
      ) : null}

      <section className="space-y-3">
        <h2 className="text-sm font-semibold tracking-tight">{t('session.quizReview')}</h2>
        {plan.quiz.map((question, index) => {
          const answer = answersByQuestion.get(question.id);
          const correct = answer?.correct ?? false;
          const selected = answer?.selected ?? [];
          return (
            <Card key={question.id} className={cn(!correct && 'border-destructive/40')}>
              <CardContent className="space-y-3 py-4">
                <div className="flex items-start gap-3">
                  <span
                    className={cn(
                      'mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full',
                      correct ? 'bg-primary text-primary-foreground' : 'bg-destructive text-white',
                    )}
                  >
                    {correct ? <Check className="size-3.5" /> : <X className="size-3.5" />}
                  </span>
                  <p className="text-sm font-medium leading-snug">
                    {index + 1}. {question.question}
                  </p>
                </div>
                <ul className="space-y-1.5 pl-9">
                  {question.options.map((option, optionIndex) => {
                    const isCorrect = question.correct.includes(optionIndex);
                    const isChosen = selected.includes(optionIndex);
                    return (
                      <li
                        // biome-ignore lint/suspicious/noArrayIndexKey: options are addressed by position.
                        key={optionIndex}
                        className={cn(
                          'rounded-2xl px-3 py-1.5 text-sm',
                          isCorrect && 'bg-primary/10 font-medium',
                          isChosen && !isCorrect && 'bg-destructive/10 line-through',
                        )}
                      >
                        {option}
                      </li>
                    );
                  })}
                </ul>
                {question.explanation ? (
                  <p className="pl-9 text-sm leading-relaxed text-muted-foreground">
                    {question.explanation}
                  </p>
                ) : null}
              </CardContent>
            </Card>
          );
        })}
      </section>
    </div>
  );
}

function PointList({
  icon,
  title,
  items,
}: {
  icon: 'check' | 'cross';
  title: string;
  items: string[];
}) {
  return (
    <div className="space-y-1.5">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{title}</p>
      <ul className="space-y-1">
        {items.map((item) => (
          <li key={item} className="flex items-start gap-2 text-sm leading-snug">
            {icon === 'check' ? (
              <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
            ) : (
              <X className="mt-0.5 size-3.5 shrink-0 text-destructive" />
            )}
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
