import {
  ArrowLeft,
  BookOpenCheck,
  BrainCircuit,
  CalendarClock,
  FileText,
  ListChecks,
  PenLine,
  SkipForward,
  Timer,
} from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { MINUTES_PER_TOPIC } from '@/features/study/lib/study';
import type { MessageId, Translate } from '@/i18n';
import { useT } from '@/i18n';

/**
 * What the app does, in the order the student meets it.
 *
 * Shown once straight after setup — with a skip, because someone who already knows what a
 * spaced-repetition study app is should not have to read this — and available from the
 * header afterwards.
 */
export function GuideScreen({
  onDone,
  firstRun,
}: {
  onDone: () => void;
  /** After setup the button reads as "start"; from the header it reads as "back". */
  firstRun: boolean;
}) {
  const t = useT();
  return (
    <div className="mx-auto w-full max-w-3xl space-y-8 p-8">
      <header className="space-y-3">
        <div className="flex items-center justify-between">
          {firstRun ? (
            <span className="text-xs uppercase tracking-wide text-muted-foreground">
              {t('guide.firstRun')}
            </span>
          ) : (
            <Button variant="ghost" size="sm" onClick={onDone} className="-ml-2">
              <ArrowLeft className="size-4" />
              {t('common.back')}
            </Button>
          )}
          {/* Reachable without scrolling: someone who already knows what a
              spaced-repetition study app is should not have to read to the bottom. */}
          {firstRun ? (
            <Button variant="ghost" size="sm" onClick={onDone}>
              {t('common.skip')}
              <SkipForward className="size-4" />
            </Button>
          ) : null}
        </div>
        <h1 className="text-2xl font-semibold tracking-tight">{t('guide.title')}</h1>
        <p className="text-sm leading-relaxed text-muted-foreground">{t('guide.blurb')}</p>
      </header>

      <section className="space-y-3">
        <Step
          number={1}
          icon={<FileText className="size-5 text-primary" />}
          title={t('guide.step1.title')}
          body={t('guide.step1.body')}
        />
        <Step
          number={2}
          icon={<BrainCircuit className="size-5 text-primary" />}
          title={t('guide.step2.title')}
          body={t('guide.step2.body')}
        />
        <Step
          number={3}
          icon={<BookOpenCheck className="size-5 text-primary" />}
          title={t('guide.step3.title')}
          body={t('guide.step3.body')}
        />
        <Step
          number={4}
          icon={<CalendarClock className="size-5 text-primary" />}
          title={t('guide.step4.title')}
          body={t('guide.step4.body')}
        />
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <PenLine className="size-4 text-muted-foreground" />
          {t('guide.pace')}
        </h2>
        <Card>
          <CardContent className="overflow-x-auto p-0">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left text-xs uppercase tracking-wide text-muted-foreground">
                  <th className="px-4 py-2.5 font-medium">{t('guide.pace.col.pace')}</th>
                  <th className="px-4 py-2.5 font-medium">{t('guide.pace.col.reading')}</th>
                  <th className="px-4 py-2.5 font-medium">{t('guide.pace.col.answer')}</th>
                  <th className="px-4 py-2.5 text-right font-medium">
                    {t('guide.pace.col.minutes')}
                  </th>
                </tr>
              </thead>
              <tbody>
                <Pace
                  t={t}
                  mode="full"
                  label="study.pace.full"
                  reading="guide.pace.readingFull"
                  answer="guide.pace.answerEssay"
                />
                <Pace
                  t={t}
                  mode="balanced"
                  label="study.pace.balanced"
                  reading="guide.pace.readingFull"
                  answer="guide.pace.answerList"
                />
                <Pace
                  t={t}
                  mode="sprint"
                  label="study.pace.sprint"
                  reading="guide.pace.readingDigest"
                  answer="guide.pace.answerList"
                />
              </tbody>
            </table>
          </CardContent>
        </Card>
        <p className="text-xs leading-relaxed text-muted-foreground">{t('guide.pace.note')}</p>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <ListChecks className="size-4 text-muted-foreground" />
          {t('guide.selection')}
        </h2>
        <div className="grid gap-3 sm:grid-cols-3">
          <Tile title={t('study.pick.auto')} body={t('study.pick.auto.blurb')} />
          <Tile title={t('study.pick.spread')} body={t('study.pick.spread.blurb')} />
          <Tile title={t('study.pick.manual')} body={t('guide.selection.manual')} />
        </div>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <CalendarClock className="size-4 text-muted-foreground" />
          {t('guide.ladder')}
        </h2>
        <Card>
          <CardContent className="space-y-3 py-5 text-sm leading-relaxed">
            <div className="flex flex-wrap items-center gap-2">
              {[1, 2, 4, 7, 14, 30].map((days, index) => (
                <span key={days} className="flex items-center gap-2">
                  {index > 0 ? <span className="text-border">→</span> : null}
                  <Badge variant="outline" className="tabular-nums">
                    {t('guide.ladder.days', { days })}
                  </Badge>
                </span>
              ))}
            </div>
            <p className="text-muted-foreground">{t('guide.ladder.body')}</p>
          </CardContent>
        </Card>
      </section>

      <section className="space-y-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold tracking-tight">
          <Timer className="size-4 text-muted-foreground" />
          {t('guide.details')}
        </h2>
        <ul className="space-y-2 text-sm leading-relaxed text-muted-foreground">
          <li>{t('guide.details.resume')}</li>
          <li>{t('guide.details.timer')}</li>
          <li>{t('guide.details.models')}</li>
          <li>{t('guide.details.files')}</li>
        </ul>
      </section>

      <div className="flex justify-end border-t border-border pt-6">
        <Button onClick={onDone}>{firstRun ? t('guide.start') : t('common.close')}</Button>
      </div>
    </div>
  );
}

function Step({
  number,
  icon,
  title,
  body,
}: {
  number: number;
  icon: React.ReactNode;
  title: string;
  body: string;
}) {
  return (
    <Card>
      <CardContent className="flex gap-4 py-5">
        <div className="flex flex-col items-center gap-2">
          {icon}
          <span className="font-mono text-xs text-muted-foreground">{number}</span>
        </div>
        <div className="min-w-0 flex-1 space-y-1">
          <p className="text-sm font-medium">{title}</p>
          <p className="text-sm leading-relaxed text-muted-foreground">{body}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function Pace({
  mode,
  label,
  reading,
  answer,
  t,
}: {
  mode: keyof typeof MINUTES_PER_TOPIC;
  label: MessageId;
  reading: MessageId;
  answer: MessageId;
  t: Translate;
}) {
  return (
    <tr className="border-b border-border last:border-0">
      <td className="px-4 py-2.5 font-medium">{t(label)}</td>
      <td className="px-4 py-2.5 text-muted-foreground">{t(reading)}</td>
      <td className="px-4 py-2.5 text-muted-foreground">{t(answer)}</td>
      <td className="px-4 py-2.5 text-right tabular-nums text-muted-foreground">
        ≈{MINUTES_PER_TOPIC[mode]}
      </td>
    </tr>
  );
}

function Tile({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-4xl border border-border px-4 py-3">
      <p className="text-sm font-medium">{title}</p>
      <p className="mt-1 text-xs leading-relaxed text-muted-foreground">{body}</p>
    </div>
  );
}
