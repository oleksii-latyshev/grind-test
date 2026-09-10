import { BookOpen, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import type { Translate } from '@/i18n';
import { useT } from '@/i18n';
import { usePreference } from '@/lib/prefs';
import type { StudyOverview } from '@/lib/types';
import { Segment } from './Segment';

/** Multiples of three, so every batch still draws from the beginning, middle and end. */
const BATCH_SIZES = [3, 6, 9];

const isBatchSize = (value: unknown): value is number =>
  typeof value === 'number' && BATCH_SIZES.includes(value);

function readingStatus(t: Translate, study: StudyOverview): string {
  if (study.available === 0) return t('study.needNotes');
  if (study.new === 0) return t('reading.allRead');
  return t('reading.left', { count: study.new });
}

interface Props {
  study: StudyOverview;
  busy: boolean;
  onStart: (size: number) => void;
}

export function ReadingCard({ study, busy, onStart }: Props) {
  const t = useT();
  const [size, setSize] = usePreference('grind:reading-batch', 3, isBatchSize);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <BookOpen className="size-5 text-primary" />
          {t('reading.title')}
        </CardTitle>
        <CardDescription>{t('reading.blurb')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="space-y-2">
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            {t('study.size')}
          </p>
          <div className="flex flex-wrap gap-2">
            {BATCH_SIZES.map((value) => (
              <Segment key={value} active={size === value} onClick={() => setSize(value)}>
                {value}
              </Segment>
            ))}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <Button
            onClick={() => onStart(size)}
            disabled={study.available === 0 || busy || study.new === 0}
          >
            {busy ? <Loader2 className="size-4 animate-spin" /> : null}
            {t('reading.start')}
          </Button>
          <p className="text-sm text-muted-foreground">{readingStatus(t, study)}</p>
        </div>
      </CardContent>
    </Card>
  );
}
