import { ChevronLeft, Loader2, Sparkles } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { useT } from '@/i18n';
import type { Question } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  questions: Question[];
  selections: Record<string, number[]>;
  submitting: boolean;
  onChoose: (questionId: string, option: number, multi: boolean) => void;
  onBack: () => void;
  onSubmit: () => void;
}

export function QuizStep({ questions, selections, submitting, onChoose, onBack, onSubmit }: Props) {
  const t = useT();
  const answered = Object.values(selections).filter((chosen) => chosen.length > 0).length;

  return (
    <section className="space-y-5">
      <div className="space-y-1">
        <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
          <Sparkles className="size-5 text-primary" />
          {t('session.quiz.title')}
        </h2>
        <p className="text-sm text-muted-foreground">
          {t('session.quiz.answered', { answered, total: questions.length })}
        </p>
      </div>

      {questions.map((question, index) => {
        const chosen = selections[question.id] ?? [];
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
                        onClick={() => onChoose(question.id, optionIndex, multi)}
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
        <Button variant="outline" onClick={onBack}>
          <ChevronLeft className="size-4" />
          {t('session.backToOpen')}
        </Button>
        <Button onClick={onSubmit} disabled={submitting}>
          {submitting ? <Loader2 className="size-4 animate-spin" /> : null}
          {t('session.finish')}
        </Button>
      </div>
    </section>
  );
}
