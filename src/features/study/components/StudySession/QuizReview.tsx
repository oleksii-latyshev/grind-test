import { Check, X } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { useT } from '@/i18n';
import type { AnswerRecord, Question } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  questions: Question[];
  answers: AnswerRecord[];
}

export function QuizReview({ questions, answers }: Props) {
  const t = useT();
  const answersByQuestion = new Map(answers.map((answer) => [answer.question_id, answer]));

  return (
    <section className="space-y-3">
      <h2 className="text-sm font-semibold tracking-tight">{t('session.quizReview')}</h2>
      {questions.map((question, index) => {
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
  );
}
