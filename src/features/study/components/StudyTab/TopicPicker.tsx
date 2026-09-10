import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { STAGES, stageOf } from '@/features/study/lib/study';
import { useT } from '@/i18n';
import type { SubjectDetail, Topic } from '@/lib/types';
import { cn } from '@/lib/utils';

interface Props {
  detail: SubjectDetail;
  chosen: string[];
  maxTopics: number;
  query: string;
  onQuery: (value: string) => void;
  onToggle: (topic: Topic, checked: boolean) => void;
}

export function TopicPicker({ detail, chosen, maxTopics, query, onQuery, onToggle }: Props) {
  const t = useT();
  const needle = query.trim().toLowerCase();
  const withNotes = new Set(detail.knowledge_topic_ids);
  const atCap = chosen.length >= maxTopics;

  const sections = detail.subject.sections
    .map((section) => ({
      title: section.title,
      topics: section.topics.filter(
        (topic) =>
          withNotes.has(topic.id) &&
          (needle === '' ||
            topic.title.toLowerCase().includes(needle) ||
            topic.id.toLowerCase().includes(needle)),
      ),
    }))
    .filter((section) => section.topics.length > 0);

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <Input
          value={query}
          onChange={(event) => onQuery(event.target.value)}
          placeholder={t('study.search')}
          className="max-w-xs"
        />
        <p className="text-xs text-muted-foreground">
          {t('study.chosen', { count: chosen.length, max: maxTopics })}
        </p>
      </div>

      <ScrollArea className="h-80 rounded-4xl border border-border">
        {sections.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">{t('study.nothingFound')}</p>
        ) : (
          sections.map((section) => (
            <div key={section.title}>
              <p className="sticky top-0 z-10 border-b border-border bg-background/95 px-4 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground backdrop-blur">
                {section.title}
              </p>
              {section.topics.map((topic) => {
                const checked = chosen.includes(topic.id);
                const stage = stageOf(detail.topic_study[topic.id]);
                return (
                  <label
                    key={topic.id}
                    className={cn(
                      'flex cursor-pointer items-start gap-3 px-4 py-2.5 transition-colors hover:bg-muted/50',
                      !checked && atCap && 'cursor-not-allowed opacity-50',
                    )}
                  >
                    <Checkbox
                      checked={checked}
                      disabled={!checked && atCap}
                      onCheckedChange={(value) => onToggle(topic, value === true)}
                      className="mt-0.5"
                    />
                    <span className="min-w-0 flex-1 space-y-0.5">
                      <span className="flex items-center gap-2">
                        <Badge variant="outline" className="font-mono text-[0.65rem]">
                          {topic.id}
                        </Badge>
                        <span className={cn('text-[0.7rem]', STAGES[stage].className)}>
                          {t(STAGES[stage].label)}
                        </span>
                      </span>
                      <span className="block text-sm leading-snug">{topic.title}</span>
                    </span>
                  </label>
                );
              })}
            </div>
          ))
        )}
      </ScrollArea>
    </div>
  );
}
