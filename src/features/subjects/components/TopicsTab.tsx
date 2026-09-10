import { CheckCircle2, Circle, Loader2, Sparkles, TriangleAlert } from 'lucide-react';
import { useMemo, useState } from 'react';
import { toast } from 'sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Progress } from '@/components/ui/progress';
import { STAGES, stageOf } from '@/features/study/lib/study';
import type { Translate } from '@/i18n';
import { useT } from '@/i18n';
import { api, errorMessage, onGenerationProgress } from '@/lib/api';
import type { Stage, SubjectDetail, Topic } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  detail: SubjectDetail;
  onRefresh: () => void;
  onOpenTopic: (topic: Topic) => void;
}

interface RunState {
  done: number;
  total: number;
  active: string[];
  failed: string[];
}

export function TopicsTab({ detail, onRefresh, onOpenTopic }: Props) {
  const t = useT();
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [run, setRun] = useState<RunState | null>(null);

  const covered = useMemo(() => new Set(detail.knowledge_topic_ids), [detail.knowledge_topic_ids]);
  const missing = useMemo(
    () => detail.subject.sections.flatMap((s) => s.topics).filter((t) => !covered.has(t.id)),
    [detail.subject, covered],
  );

  function toggle(topicId: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(topicId)) next.delete(topicId);
      else next.add(topicId);
      return next;
    });
  }

  async function generate() {
    // An empty selection means "everything still missing" — the common case.
    const topicIds = selected.size > 0 ? [...selected] : null;
    setRun({ done: 0, total: topicIds?.length ?? missing.length, active: [], failed: [] });

    const unlisten = await onGenerationProgress((event) => {
      setRun((current) => {
        if (!current) return current;
        switch (event.kind) {
          case 'started':
            return { ...current, total: event.total };
          case 'topic_started':
            return { ...current, active: [...current.active, event.topic_id] };
          case 'topic_done':
            return {
              ...current,
              done: event.done,
              active: current.active.filter((id) => id !== event.topic_id),
            };
          case 'topic_failed':
            return {
              ...current,
              active: current.active.filter((id) => id !== event.topic_id),
              failed: [...current.failed, `${event.topic_id}: ${event.error}`],
            };
          default:
            return current;
        }
      });
    });

    try {
      const report = await api.generateKnowledge({ subjectId: detail.subject.id, topicIds });
      toast.success(
        report.failed
          ? t('topics.doneWithErrors', {
              generated: report.generated,
              failed: report.failed,
            })
          : t('topics.done', { generated: report.generated }),
      );
      setSelected(new Set());
      onRefresh();
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      unlisten();
      setRun(null);
    }
  }

  const busy = run !== null;

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">
          {t('topics.coverage', {
            covered: covered.size,
            total: covered.size + missing.length,
          })}
          {selected.size > 0 ? t('topics.selected', { count: selected.size }) : ''}
        </p>
        <div className="flex gap-2">
          {selected.size > 0 ? (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSelected(new Set())}
              disabled={busy}
            >
              {t('topics.clear')}
            </Button>
          ) : null}
          <Button
            size="sm"
            onClick={generate}
            disabled={busy || (selected.size === 0 && missing.length === 0)}
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : <Sparkles className="size-4" />}
            {selected.size > 0
              ? t('topics.generateSelected', { count: selected.size })
              : t('topics.generateMissing', { count: missing.length })}
          </Button>
        </div>
      </div>

      {run ? (
        <div className="space-y-2 rounded-4xl border border-border bg-card p-4">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>{t('topics.running')}</span>
            <span className="font-mono">
              {run.done}/{run.total}
            </span>
          </div>
          <Progress value={run.total ? (run.done / run.total) * 100 : 0} />
          {run.active.length > 0 ? (
            <p className="font-mono text-xs text-muted-foreground">
              {t('topics.active', { ids: run.active.join(', ') })}
            </p>
          ) : null}
          {run.failed.length > 0 ? (
            <p className="text-xs text-destructive">
              {t('topics.failedCount', { count: run.failed.length })}
            </p>
          ) : null}
        </div>
      ) : null}

      {run?.failed.length ? (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertTitle>{t('topics.failed.title')}</AlertTitle>
          <AlertDescription>
            <ul className="list-disc pl-4">
              {run.failed.slice(0, 5).map((message) => (
                <li key={message} className="truncate">
                  {message}
                </li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      ) : null}

      <div className="space-y-6">
        {detail.subject.sections.map((section) => (
          <section key={section.index} className="space-y-1">
            <h3 className="sticky top-0 z-10 bg-background/90 py-2 text-sm font-semibold tracking-tight backdrop-blur">
              {section.title}
            </h3>
            <ul className="divide-y divide-border rounded-4xl border border-border">
              {section.topics.map((topic) => {
                const has = covered.has(topic.id);
                return (
                  <li
                    key={topic.id}
                    className="flex items-start gap-3 px-4 py-2.5 first:rounded-t-4xl last:rounded-b-4xl hover:bg-muted/50"
                  >
                    <Checkbox
                      className="mt-0.5"
                      checked={selected.has(topic.id)}
                      onCheckedChange={() => toggle(topic.id)}
                      disabled={busy}
                      aria-label={t('topics.select', { id: topic.id })}
                    />
                    <button
                      type="button"
                      className="flex flex-1 items-start gap-3 text-left"
                      onClick={() => onOpenTopic(topic)}
                    >
                      <Badge variant="outline" className="mt-px shrink-0 font-mono text-[0.7rem]">
                        {topic.index}
                      </Badge>
                      <span className={cn('text-sm leading-snug', !has && 'text-muted-foreground')}>
                        {topic.title}
                      </span>
                    </button>
                    <StageMark stage={stageOf(detail.topic_study[topic.id])} hasNote={has} t={t} />
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}

/** Knowledge presence plus how far the student has got with the topic. */
function StageMark({ stage, hasNote, t }: { stage: Stage; hasNote: boolean; t: Translate }) {
  if (!hasNote) {
    return <Circle className="mt-0.5 size-4 shrink-0 text-muted-foreground/40" />;
  }
  return (
    <span className="mt-px flex shrink-0 items-center gap-1.5">
      <span className={cn('text-xs', STAGES[stage].className)}>{t(STAGES[stage].label)}</span>
      <CheckCircle2 className={cn('size-4', STAGES[stage].className)} />
    </span>
  );
}
