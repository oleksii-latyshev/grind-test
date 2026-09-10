import {
  ArrowRight,
  ChevronLeft,
  Loader2,
  Repeat,
  RotateCw,
  TriangleAlert,
  Zap,
} from 'lucide-react';
import { NoteReader } from '@/components/NoteReader';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { MessageId } from '@/i18n';
import { useT } from '@/i18n';
import type { SessionPlan } from '@/lib/types';

interface Props {
  plan: SessionPlan;
  readIndex: number;
  /** The last note is done and the questions have not landed yet. */
  waiting: boolean;
  prepareError: string | null;
  onPrevious: () => void;
  onNext: () => void;
  onRetry: () => void;
}

function nextLabel(isLastTopic: boolean, waiting: boolean): MessageId {
  if (!isLastTopic) return 'session.nextTopic';
  return waiting ? 'session.preparing' : 'session.toQuestions';
}

export function ReadingStep({
  plan,
  readIndex,
  waiting,
  prepareError,
  onPrevious,
  onNext,
  onRetry,
}: Props) {
  const t = useT();
  const entry = plan.topics[readIndex];
  const note = plan.notes[readIndex];
  const isLastTopic = readIndex + 1 === plan.topics.length;

  return (
    <>
      <NoteReader
        note={{
          ...note,
          // A sprint reads the digest; older sessions have no `reading` array.
          body: plan.reading[readIndex] ?? note.body,
        }}
        eyebrow={
          <>
            <Badge variant="outline" className="font-mono text-[0.7rem]">
              {entry.topic.id}
            </Badge>
            <span className="text-xs text-muted-foreground">
              {t('session.topicOf', { index: readIndex + 1, total: plan.topics.length })}
            </span>
            {plan.mode === 'sprint' ? (
              <Badge variant="secondary" className="gap-1">
                <Zap className="size-3" />
                {t('session.digest')}
              </Badge>
            ) : null}
            {entry.is_review ? (
              <Badge variant="secondary" className="gap-1">
                <Repeat className="size-3" />
                {t('session.review')}
              </Badge>
            ) : null}
          </>
        }
      />
      <div className="mx-auto flex w-full max-w-3xl items-center justify-between border-t border-border pt-6">
        <Button variant="outline" onClick={onPrevious} disabled={readIndex === 0}>
          <ChevronLeft className="size-4" />
          {t('session.prev')}
        </Button>
        {prepareError && isLastTopic ? (
          <Button variant="outline" onClick={onRetry}>
            <RotateCw className="size-4" />
            {t('common.retry')}
          </Button>
        ) : (
          <Button onClick={onNext} disabled={waiting}>
            {waiting ? <Loader2 className="size-4 animate-spin" /> : null}
            {t(nextLabel(isLastTopic, waiting))}
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
  );
}
