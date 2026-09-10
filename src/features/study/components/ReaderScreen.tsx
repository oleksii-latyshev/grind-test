import { ArrowLeft, ArrowRight, Check, ChevronLeft, FileWarning, Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { NoteReader } from '@/components/NoteReader';
import { ReadingSizeControl } from '@/components/ReadingSizeControl';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { KnowledgeNote, Topic } from '@/lib/types';

interface Props {
  topics: Topic[];
  onBack: () => void;
  /** Notes-only reading: fetch the next batch once this one is read. Empty means none left. */
  onMore?: () => Promise<Topic[]>;
}

/** Reading without questions — one topic from the topic list, or a notes-only batch. */
export function ReaderScreen({ topics: initial, onBack, onMore }: Props) {
  const t = useT();
  const [topics, setTopics] = useState(initial);
  const [index, setIndex] = useState(0);
  const [note, setNote] = useState<KnowledgeNote | null>(null);
  const [loading, setLoading] = useState(true);
  const [marked, setMarked] = useState(false);
  const [fetching, setFetching] = useState(false);
  const topic = topics[index];
  const isLast = index + 1 === topics.length;

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setMarked(false);
    api
      .getKnowledge(topic.subject, topic.id)
      .then((result) => {
        if (!cancelled) setNote(result);
      })
      .catch((error) => {
        if (!cancelled) toast.error(errorMessage(error));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [topic]);

  async function markRead() {
    try {
      await api.markTopicRead(topic.id);
      setMarked(true);
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  function go(next: number) {
    setIndex(next);
    window.scrollTo({ top: 0 });
  }

  function nextTopic() {
    // Fire and forget, as in a study session: bookkeeping must not block reading.
    api.markTopicRead(topic.id).catch(() => {});
    go(index + 1);
  }

  async function more() {
    if (!onMore) return;
    setFetching(true);
    try {
      // Marked first, so the next batch cannot hand this topic straight back.
      if (!marked) await api.markTopicRead(topic.id);
      const next = await onMore();
      if (next.length === 0) {
        setMarked(true);
        toast.info(t('reading.allRead'));
        return;
      }
      setTopics(next);
      go(0);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setFetching(false);
    }
  }

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-8">
      {/* Sits directly under the app header so the size control stays reachable while
          scrolling a thousand-word note. */}
      <div className="sticky top-12 z-10 -mx-8 flex items-center justify-between border-b border-border bg-background/95 px-8 py-2 backdrop-blur">
        <Button variant="ghost" size="sm" onClick={onBack} className="-ml-2">
          <ArrowLeft className="size-4" />
          {t('common.back')}
        </Button>
        <ReadingSizeControl />
      </div>

      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          {t('reader.loading')}
        </div>
      ) : note ? (
        <>
          <NoteReader
            note={note}
            eyebrow={
              <>
                <Badge variant="outline" className="font-mono text-[0.7rem]">
                  {topic.id}
                </Badge>
                {topics.length > 1 ? (
                  <span className="text-xs text-muted-foreground">
                    {t('session.topicOf', { index: index + 1, total: topics.length })}
                  </span>
                ) : null}
              </>
            }
          />
          <div className="mx-auto flex w-full max-w-3xl flex-wrap items-center justify-end gap-3 border-t border-border pt-6">
            {topics.length > 1 ? (
              <Button
                variant="outline"
                onClick={() => go(index - 1)}
                disabled={index === 0}
                className="mr-auto"
              >
                <ChevronLeft className="size-4" />
                {t('session.prev')}
              </Button>
            ) : null}
            {!isLast ? (
              <Button onClick={nextTopic}>
                {t('session.nextTopic')}
                <ArrowRight className="size-4" />
              </Button>
            ) : (
              <>
                <Button
                  variant={marked || onMore ? 'outline' : 'default'}
                  onClick={markRead}
                  disabled={marked}
                >
                  <Check className="size-4" />
                  {marked ? t('reader.marked') : t('reader.markRead')}
                </Button>
                {onMore ? (
                  <Button onClick={more} disabled={fetching}>
                    {fetching ? <Loader2 className="size-4 animate-spin" /> : null}
                    {t('reading.more')}
                    <ArrowRight className="size-4" />
                  </Button>
                ) : null}
              </>
            )}
          </div>
        </>
      ) : (
        <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
          <FileWarning className="size-6" />
          <p>{t('reader.missing')}</p>
        </div>
      )}
    </div>
  );
}
