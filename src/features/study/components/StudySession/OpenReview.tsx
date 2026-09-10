import { Check, X } from 'lucide-react';
import { Card, CardContent } from '@/components/ui/card';
import { scoreTone } from '@/features/study/lib/study';
import { useT } from '@/i18n';
import type { OpenGrading, OpenQuestion } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  questions: OpenQuestion[];
  gradings: OpenGrading[];
  answers: Record<string, string>;
}

export function OpenReview({ questions, gradings, answers }: Props) {
  const t = useT();

  return (
    <section className="space-y-4">
      <h2 className="text-sm font-semibold tracking-tight">{t('session.openReview')}</h2>
      {gradings.map((grading) => {
        const question = questions.find((entry) => entry.topic_id === grading.topic_id);
        const answer = answers[grading.topic_id];
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

              {answer?.trim() ? (
                <details className="text-sm">
                  <summary className="cursor-pointer text-xs uppercase tracking-wide text-muted-foreground">
                    {t('session.yourAnswer')}
                  </summary>
                  <p className="mt-2 whitespace-pre-wrap leading-relaxed text-muted-foreground">
                    {answer}
                  </p>
                </details>
              ) : null}
            </CardContent>
          </Card>
        );
      })}
    </section>
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
