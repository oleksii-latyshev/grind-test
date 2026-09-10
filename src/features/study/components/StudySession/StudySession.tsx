import { X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { ReadingSizeControl } from '@/components/ReadingSizeControl';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { clearDraft, readDraft, writeDraft } from '@/features/study/lib/drafts';
import { sessionReady } from '@/features/study/lib/study';
import type { MessageId } from '@/i18n';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { SessionPlan, SessionResult } from '@/lib/types';
import { cn } from '@/lib/utils';
import { OpenStep } from './OpenStep';
import { QuizStep } from './QuizStep';
import { ReadingStep } from './ReadingStep';
import { SessionSummary } from './SessionSummary';

type Step = 'read' | 'open' | 'quiz' | 'done';

const STEP_LABELS: Record<Exclude<Step, 'done'>, MessageId> = {
  read: 'session.step.read',
  open: 'session.step.open',
  quiz: 'session.step.quiz',
};

/** Reading fills the first 60% of the bar, one note at a time. */
function progressOf(step: Step, readIndex: number, topicCount: number): number {
  if (step === 'read') return ((readIndex + 1) / topicCount) * 60;
  return step === 'open' ? 75 : 90;
}

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
      />
    );
  }

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
        <Progress value={progressOf(step, readIndex, plan.topics.length)} />
      </header>

      {step === 'read' ? (
        <ReadingStep
          plan={plan}
          readIndex={readIndex}
          waiting={waiting}
          prepareError={prepareError}
          onPrevious={() => setReadIndex((value) => value - 1)}
          onNext={advanceReading}
          onRetry={() => {
            setPrepareError(null);
            setRetry((value) => value + 1);
          }}
        />
      ) : null}

      {step === 'open' ? (
        <OpenStep
          plan={plan}
          answers={openAnswers}
          onAnswer={(topicId, answer) =>
            setOpenAnswers((current) => ({ ...current, [topicId]: answer }))
          }
          onBack={() => setStep('read')}
          onNext={() => setStep('quiz')}
        />
      ) : null}

      {step === 'quiz' ? (
        <QuizStep
          questions={plan.quiz}
          selections={quizSelections}
          submitting={submitting}
          onChoose={chooseOption}
          onBack={() => setStep('open')}
          onSubmit={submit}
        />
      ) : null}
    </div>
  );
}
