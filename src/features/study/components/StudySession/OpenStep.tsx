import { ArrowRight, ChevronLeft, PenLine } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Textarea } from '@/components/ui/textarea';
import { useT } from '@/i18n';
import type { SessionPlan } from '@/lib/types';

interface Props {
  plan: SessionPlan;
  answers: Record<string, string>;
  onAnswer: (topicId: string, answer: string) => void;
  onBack: () => void;
  onNext: () => void;
}

export function OpenStep({ plan, answers, onAnswer, onBack, onNext }: Props) {
  const t = useT();
  // Every pace but the full one asks for a short list instead of prose.
  const listAnswer = plan.mode !== 'full';

  return (
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
              value={answers[question.topic_id] ?? ''}
              onChange={(event) => onAnswer(question.topic_id, event.target.value)}
              placeholder={
                listAnswer ? t('session.open.listPlaceholder') : t('session.open.placeholder')
              }
            />
          </CardContent>
        </Card>
      ))}

      <div className="flex justify-between border-t border-border pt-6">
        <Button variant="outline" onClick={onBack}>
          <ChevronLeft className="size-4" />
          {t('session.toNotes')}
        </Button>
        <Button onClick={onNext}>
          {t('session.toQuiz')}
          <ArrowRight className="size-4" />
        </Button>
      </div>
    </section>
  );
}
