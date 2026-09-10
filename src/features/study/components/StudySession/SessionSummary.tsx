import { ArrowRight, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { readSessionSettings } from '@/features/study/lib/session-settings';
import { formatDue, STAGES, scoreTone } from '@/features/study/lib/study';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { SessionPlan, SessionResult } from '@/lib/types';
import { cn } from '@/lib/utils';
import { OpenReview } from './OpenReview';
import { QuizReview } from './QuizReview';

interface Props {
  plan: SessionPlan;
  result: SessionResult;
  openAnswers: Record<string, string>;
  onDone: () => void;
  onContinue: (plan: SessionPlan) => void;
}

export function SessionSummary({ plan, result, openAnswers, onDone, onContinue }: Props) {
  const t = useT();
  const [starting, setStarting] = useState(false);

  // Read from storage rather than passed down: the same settings the study tab is showing.
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
        <OpenReview
          questions={plan.open_questions}
          gradings={result.gradings}
          answers={openAnswers}
        />
      ) : null}

      <QuizReview questions={plan.quiz} answers={result.answers} />
    </div>
  );
}
