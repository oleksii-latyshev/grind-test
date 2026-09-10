import { ChevronLeft, ChevronRight, Lightbulb, Loader2, X } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import type { MessageId } from '@/i18n';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { AttemptResult, Hint, Quiz } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  quiz: Quiz;
  onExit: () => void;
  onFinish: (result: AttemptResult, quiz: Quiz) => void;
}

export function QuizScreen({ quiz, onExit, onFinish }: Props) {
  const t = useT();
  const [index, setIndex] = useState(0);
  const [selections, setSelections] = useState<Record<string, number[]>>({});
  const [hints, setHints] = useState<Record<string, Hint>>({});
  const [loadingHint, setLoadingHint] = useState(false);
  const [submitting, setSubmitting] = useState(false);

  const question = quiz.questions[index];
  const chosen = selections[question.id] ?? [];
  const isLast = index === quiz.questions.length - 1;
  const answered = Object.values(selections).filter((s) => s.length > 0).length;

  function choose(option: number) {
    setSelections((current) => {
      const previous = current[question.id] ?? [];
      if (question.type === 'single') {
        return { ...current, [question.id]: [option] };
      }
      const next = previous.includes(option)
        ? previous.filter((value) => value !== option)
        : [...previous, option];
      return { ...current, [question.id]: next };
    });
  }

  async function requestHint() {
    setLoadingHint(true);
    try {
      const hint = await api.getHint(quiz.subject, quiz.id, question.id);
      setHints((current) => ({ ...current, [question.id]: hint }));
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setLoadingHint(false);
    }
  }

  async function submit() {
    setSubmitting(true);
    try {
      const result = await api.submitQuiz({
        subjectId: quiz.subject,
        quizId: quiz.id,
        selections,
        hintsUsed: Object.keys(hints),
      });
      onFinish(result, quiz);
    } catch (error) {
      toast.error(errorMessage(error));
      setSubmitting(false);
    }
  }

  const hint = hints[question.id];

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-8">
      <header className="space-y-3">
        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            {t('quiz.position', {
              index: index + 1,
              total: quiz.questions.length,
              answered,
            })}
          </p>
          <Button variant="ghost" size="sm" onClick={onExit}>
            <X className="size-4" />
            {t('common.exit')}
          </Button>
        </div>
        <Progress value={((index + 1) / quiz.questions.length) * 100} />
      </header>

      <Card>
        <CardContent className="space-y-5 py-6">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant="outline" className="font-mono text-[0.7rem]">
              {question.topic_id}
            </Badge>
            <Badge variant="secondary">{t(DIFFICULTY_LABELS[question.difficulty])}</Badge>
            {question.type === 'multi' ? (
              <Badge variant="secondary">{t('session.multi')}</Badge>
            ) : null}
          </div>

          <h2 className="text-lg font-medium leading-snug">{question.question}</h2>

          <ul className="space-y-2">
            {question.options.map((option, optionIndex) => {
              const active = chosen.includes(optionIndex);
              return (
                // biome-ignore lint/suspicious/noArrayIndexKey: options are addressed by position.
                <li key={optionIndex}>
                  <button
                    type="button"
                    onClick={() => choose(optionIndex)}
                    aria-pressed={active}
                    className={cn(
                      'flex w-full items-start gap-3 rounded-4xl border px-4 py-3 text-left text-sm transition-colors',
                      active ? 'border-primary bg-primary/10' : 'border-input hover:bg-muted/60',
                    )}
                  >
                    <span
                      className={cn(
                        'mt-px flex size-5 shrink-0 items-center justify-center border text-[0.7rem] font-medium',
                        question.type === 'multi' ? 'rounded-md' : 'rounded-full',
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

          {hint ? (
            <div className="space-y-2 rounded-4xl border border-border bg-muted/50 p-4">
              <p className="flex items-center gap-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                <Lightbulb className="size-3.5" />
                {t('quiz.hint')}
              </p>
              <p className="text-sm leading-relaxed">{hint.hint}</p>
              {hint.related_concepts.length > 0 ? (
                <div className="flex flex-wrap gap-1.5 pt-1">
                  {hint.related_concepts.map((concept) => (
                    <Badge key={concept} variant="outline" className="text-[0.7rem]">
                      {concept}
                    </Badge>
                  ))}
                </div>
              ) : null}
            </div>
          ) : (
            <Button variant="outline" size="sm" onClick={requestHint} disabled={loadingHint}>
              {loadingHint ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Lightbulb className="size-4" />
              )}
              {t('quiz.hint')}
            </Button>
          )}
        </CardContent>
      </Card>

      <footer className="flex items-center justify-between">
        <Button
          variant="outline"
          onClick={() => setIndex((value) => value - 1)}
          disabled={index === 0}
        >
          <ChevronLeft className="size-4" />
          {t('common.back')}
        </Button>

        {isLast ? (
          <Button onClick={submit} disabled={submitting}>
            {submitting ? <Loader2 className="size-4 animate-spin" /> : null}
            {t('quiz.finish')}
          </Button>
        ) : (
          <Button onClick={() => setIndex((value) => value + 1)}>
            {t('common.next')}
            <ChevronRight className="size-4" />
          </Button>
        )}
      </footer>
    </div>
  );
}

const DIFFICULTY_LABELS: Record<Quiz['difficulty'], MessageId> = {
  easy: 'difficulty.easy',
  medium: 'difficulty.medium',
  hard: 'difficulty.hard',
  mixed: 'difficulty.mixed',
};
