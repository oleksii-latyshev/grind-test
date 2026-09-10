import { BookOpenCheck, Loader2 } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { type TopicPick, useSessionSettings } from '@/features/study/lib/session-settings';
import { MAX_TOPICS, MINUTES_PER_TOPIC, SESSION_SIZES } from '@/features/study/lib/study';
import type { MessageId } from '@/i18n';
import { useT } from '@/i18n';
import type { api } from '@/lib/api';
import type { SessionMode, SubjectDetail, Topic } from '@/lib/types';
import { Segment } from './Segment';
import { TopicPicker } from './TopicPicker';

export type SessionRequest = Omit<Parameters<typeof api.planStudySession>[0], 'subjectId'>;

const PACE: Record<SessionMode, { label: MessageId; blurb: MessageId }> = {
  full: { label: 'study.pace.full', blurb: 'study.pace.full.blurb' },
  balanced: { label: 'study.pace.balanced', blurb: 'study.pace.balanced.blurb' },
  sprint: { label: 'study.pace.sprint', blurb: 'study.pace.sprint.blurb' },
};

const PICK: Record<TopicPick, MessageId> = {
  auto: 'study.pick.auto',
  spread: 'study.pick.spread',
  manual: 'study.pick.manual',
};

function pickBlurb(pick: TopicPick, pace: SessionMode): MessageId {
  if (pick === 'spread') return 'study.pick.spread.blurb';
  return pace === 'sprint' ? 'study.pick.sprint.blurb' : 'study.pick.auto.blurb';
}

interface Props {
  detail: SubjectDetail;
  busy: boolean;
  onStart: (request: SessionRequest) => void;
}

export function SessionCard({ detail, busy, onStart }: Props) {
  const t = useT();
  // Remembered per machine: these three are the same on almost every visit, and re-picking
  // them each time is the friction between deciding to study and studying.
  const [settings, setSettings] = useSessionSettings();
  const { pace, pick, size } = settings;
  const [chosen, setChosen] = useState<string[]>([]);
  const [query, setQuery] = useState('');
  const { study } = detail;
  const ready = study.available > 0;
  const nothingLeft = study.due_now + study.new === 0;
  const maxTopics = MAX_TOPICS[pace];
  const canStart =
    ready && !busy && (pick === 'manual' ? chosen.length > 0 : pick === 'spread' || !nothingLeft);

  function changePace(next: SessionMode) {
    // The size steps differ per pace, so the size moves with it rather than being carried
    // over into a range where it no longer appears as an option.
    setSettings({ ...settings, pace: next, size: SESSION_SIZES[next][1] });
    setChosen((current) => current.slice(0, MAX_TOPICS[next]));
  }

  function toggle(topic: Topic, checked: boolean) {
    setChosen((current) => {
      if (!checked) return current.filter((id) => id !== topic.id);
      if (current.includes(topic.id) || current.length >= maxTopics) return current;
      return [...current, topic.id];
    });
  }

  function start() {
    const manual = pick === 'manual';
    onStart({
      size: manual ? chosen.length : size,
      mode: pace,
      selection: pick === 'spread' ? 'spread' : 'scheduled',
      topicIds: manual ? chosen : undefined,
    });
  }

  function hint(): string | null {
    if (!ready) return t('study.needNotes');
    if (pick === 'auto' && nothingLeft) return t('study.allDone');
    if (pick === 'manual' && chosen.length === 0) return t('study.pickSome', { max: maxTopics });
    return null;
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BookOpenCheck className="size-5 text-primary" />
          {t('study.new.title')}
        </CardTitle>
        <CardDescription>{t('study.new.blurb')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="space-y-2">
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t('study.pace')}
          </p>
          <div className="flex flex-wrap gap-2">
            {(['full', 'balanced', 'sprint'] as const).map((value) => (
              <Segment key={value} active={pace === value} onClick={() => changePace(value)}>
                {t(PACE[value].label)}
              </Segment>
            ))}
          </div>
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t(PACE[pace].blurb)} {t('study.pace.perTopic', { minutes: MINUTES_PER_TOPIC[pace] })}
          </p>
        </div>

        <div className="space-y-2">
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t('study.pick')}
          </p>
          <div className="flex flex-wrap gap-2">
            {(['auto', 'spread', 'manual'] as const).map((value) => (
              <Segment
                key={value}
                active={pick === value}
                onClick={() => setSettings({ ...settings, pick: value })}
              >
                {t(PICK[value])}
              </Segment>
            ))}
          </div>
        </div>

        {pick === 'manual' ? (
          <TopicPicker
            detail={detail}
            chosen={chosen}
            maxTopics={maxTopics}
            query={query}
            onQuery={setQuery}
            onToggle={toggle}
          />
        ) : (
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t('study.size')}
            </p>
            <div className="flex flex-wrap gap-2">
              {SESSION_SIZES[pace].map((value) => (
                <Segment
                  key={value}
                  active={size === value}
                  onClick={() => setSettings({ ...settings, size: value })}
                >
                  {value}
                </Segment>
              ))}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t(pickBlurb(pick, pace))}
            </p>
          </div>
        )}

        <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <Button onClick={start} disabled={!canStart}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : null}
            {pick === 'manual' && chosen.length > 0
              ? t('study.start.count', { count: chosen.length })
              : t('study.start')}
          </Button>
          {hint() ? <p className="text-sm text-muted-foreground">{hint()}</p> : null}
        </div>
      </CardContent>
    </Card>
  );
}
