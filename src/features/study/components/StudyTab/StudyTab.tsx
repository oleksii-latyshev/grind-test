import { History } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { SessionPlan, SessionSummary, SubjectDetail, Topic } from '@/lib/types';
import { ProgressCard } from './ProgressCard';
import { ReadingCard } from './ReadingCard';
import { SessionCard, type SessionRequest } from './SessionCard';

interface Props {
  detail: SubjectDetail;
  onSessionStart: (plan: SessionPlan) => void;
  onReadingStart: (topics: Topic[]) => void;
}

export function StudyTab({ detail, onSessionStart, onReadingStart }: Props) {
  const t = useT();
  const [starting, setStarting] = useState(false);
  const [unfinished, setUnfinished] = useState<SessionSummary | null>(null);
  const subjectId = detail.subject.id;

  useEffect(() => {
    api
      .unfinishedSessions(subjectId)
      .then((sessions) => setUnfinished(sessions[0] ?? null))
      .catch(() => setUnfinished(null));
  }, [subjectId]);

  // One start at a time across the whole tab; a failure is shown, never thrown.
  async function run(action: () => Promise<void>) {
    setStarting(true);
    try {
      await action();
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  // Disk only: this returns before any model call, so the reader opens at once and the
  // question generation runs behind the first note.
  const startSession = (request: SessionRequest) =>
    run(async () => onSessionStart(await api.planStudySession({ subjectId, ...request })));

  const startReading = (size: number) =>
    run(async () => {
      const topics = await api.planReading(subjectId, size);
      if (topics.length === 0) toast.info(t('reading.allRead'));
      else onReadingStart(topics);
    });

  const resume = (session: SessionSummary) =>
    run(async () => onSessionStart(await api.resumeStudySession(subjectId, session.id)));

  return (
    <div className="space-y-6">
      {unfinished ? (
        <Card className="border-chart-3/40">
          <CardContent className="flex flex-wrap items-center justify-between gap-3 py-4">
            <div className="min-w-0 flex-1 space-y-1">
              <p className="flex items-center gap-2 text-sm font-medium">
                <History className="size-4 text-chart-3" />
                {t('study.unfinished')}
              </p>
              <p className="line-clamp-1 text-xs text-muted-foreground">
                {unfinished.topic_titles.join(' · ')}
              </p>
            </div>
            <Button variant="outline" onClick={() => resume(unfinished)} disabled={starting}>
              {t('common.continue')}
            </Button>
          </CardContent>
        </Card>
      ) : null}

      <ProgressCard study={detail.study} />
      <ReadingCard study={detail.study} busy={starting} onStart={startReading} />
      <SessionCard detail={detail} busy={starting} onStart={startSession} />
    </div>
  );
}
