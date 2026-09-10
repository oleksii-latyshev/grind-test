import { CalendarClock, Flame, GraduationCap } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { formatHours, MINUTES_PER_TOPIC } from '@/features/study/lib/study';
import { useT } from '@/i18n';
import type { SessionMode, StudyOverview } from '@/lib/types';
import { cn } from '@/lib/utils';

export function ProgressCard({ study }: { study: StudyOverview }) {
  const t = useT();
  // What is left to cover at least once, and what that costs at each pace.
  const remaining = study.new;
  // The headline is coverage — what the student has started. The ladder percentage moves in
  // steps of one level out of six per topic, so on its own it reads as "nothing done".
  const started = study.available - study.new;
  const coverage = study.available > 0 ? Math.round((started / study.available) * 100) : 0;
  const estimate = (mode: SessionMode) => formatHours(t, remaining * MINUTES_PER_TOPIC[mode]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <GraduationCap className="size-5 text-primary" />
          {t('study.progress.title')}
        </CardTitle>
        <CardDescription>{t('study.progress.blurb')}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-5">
        <div className="space-y-2">
          <div className="flex items-baseline justify-between">
            <span className="text-3xl font-semibold tabular-nums">{coverage}%</span>
            <span className="text-xs text-muted-foreground">
              {t('study.started', { started, total: study.available })}
            </span>
          </div>
          <Progress value={coverage} />
          <p className="text-xs text-muted-foreground">
            {t('study.retained', { percent: study.progress_percent })}
          </p>
        </div>

        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <Tally label={t('stage.new')} value={study.new} />
          <Tally label={t('stage.learning')} value={study.learning} tone="text-chart-2" />
          <Tally label={t('stage.review')} value={study.review} tone="text-chart-3" />
          <Tally label={t('stage.mastered')} value={study.mastered} tone="text-primary" />
        </div>

        <div className="flex flex-wrap gap-4 border-t border-border pt-4 text-sm">
          <span className="flex items-center gap-2">
            <CalendarClock className="size-4 text-muted-foreground" />
            {t('study.dueNow')} <strong className="tabular-nums">{study.due_now}</strong>
          </span>
          <span className="flex items-center gap-2">
            <Flame className="size-4 text-muted-foreground" />
            {t('study.today')} <strong className="tabular-nums">{study.studied_today}</strong>
          </span>
        </div>

        {remaining > 0 ? (
          <p className="text-xs leading-relaxed text-muted-foreground">
            {t('study.remaining', {
              count: remaining,
              full: estimate('full'),
              balanced: estimate('balanced'),
              sprint: estimate('sprint'),
            })}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}

function Tally({ label, value, tone }: { label: string; value: number; tone?: string }) {
  return (
    <div className="rounded-4xl border border-border px-4 py-3">
      <p className={cn('text-xl font-semibold tabular-nums', tone)}>{value}</p>
      <p className="text-xs text-muted-foreground">{label}</p>
    </div>
  );
}
