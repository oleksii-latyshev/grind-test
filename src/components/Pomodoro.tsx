import { Coffee, Pause, Play, RotateCcw, Settings2, SkipForward, Timer } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { MessageId, Translate } from '@/i18n';
import { useT } from '@/i18n';
import type { Phase, PomodoroSettings } from '@/lib/pomodoro';
import { formatClock, POMODORO_LIMITS, usePomodoro } from '@/lib/pomodoro';
import { cn } from '@/lib/utils';

const PHASES: Record<Phase, { label: MessageId; announce: MessageId; tone: string }> = {
  work: {
    label: 'pomodoro.work',
    announce: 'pomodoro.work.announce',
    tone: 'text-primary',
  },
  short: {
    label: 'pomodoro.short',
    announce: 'pomodoro.short.announce',
    tone: 'text-chart-2',
  },
  long: {
    label: 'pomodoro.long',
    announce: 'pomodoro.long.announce',
    tone: 'text-chart-3',
  },
};

/**
 * Work/rest rounds, in the app header so the clock survives moving between screens.
 *
 * Advisory by design: a finished round announces itself and stops, it never covers what is
 * on screen or interrupts a session. Cramming for six hours straight is the thing being
 * prevented, and knowing the time passed is enough to prevent it.
 */
export function Pomodoro() {
  const t = useT();
  const timer = usePomodoro((next) =>
    toast(t(PHASES[next].label), { description: t(PHASES[next].announce) }),
  );
  const [open, setOpen] = useState(false);
  const idle = !timer.running && timer.left === timer.total;

  return (
    <div className="flex items-center gap-0.5">
      <Button
        variant="ghost"
        size="sm"
        onClick={timer.running ? timer.pause : timer.start}
        className="gap-1.5 px-2"
        title={timer.running ? t('pomodoro.pause') : t('pomodoro.start')}
      >
        {timer.phase === 'work' ? (
          <Timer className={cn('size-4', !idle && PHASES.work.tone)} />
        ) : (
          <Coffee className={cn('size-4', PHASES[timer.phase].tone)} />
        )}
        <span className={cn('font-mono text-xs tabular-nums', !idle && PHASES[timer.phase].tone)}>
          {formatClock(timer.left)}
        </span>
        {timer.running ? <Pause className="size-3" /> : <Play className="size-3" />}
      </Button>

      {idle ? null : (
        <>
          <Button variant="ghost" size="icon" onClick={timer.reset} title={t('pomodoro.reset')}>
            <RotateCcw className="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon" onClick={timer.skip} title={t('pomodoro.skipPhase')}>
            <SkipForward className="size-3.5" />
          </Button>
        </>
      )}

      <Button
        variant="ghost"
        size="icon"
        onClick={() => setOpen(true)}
        title={t('pomodoro.settings')}
      >
        <Settings2 className="size-3.5" />
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('pomodoro.title')}</DialogTitle>
            <DialogDescription>
              {t('pomodoro.rounds', { round: timer.round, every: timer.settings.every })}
            </DialogDescription>
          </DialogHeader>

          <div className="grid grid-cols-2 gap-4">
            {(['work', 'short', 'long', 'every'] as const).map((field) => (
              <Field
                key={field}
                field={field}
                settings={timer.settings}
                onChange={timer.setSettings}
                t={t}
              />
            ))}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

const FIELD_LABELS: Record<keyof PomodoroSettings, MessageId> = {
  work: 'pomodoro.field.work',
  short: 'pomodoro.field.short',
  long: 'pomodoro.field.long',
  every: 'pomodoro.field.every',
};

function Field({
  field,
  settings,
  onChange,
  t,
}: {
  field: keyof PomodoroSettings;
  settings: PomodoroSettings;
  onChange: (settings: PomodoroSettings) => void;
  t: Translate;
}) {
  const [min, max] = POMODORO_LIMITS[field];
  return (
    <div className="space-y-1.5">
      <Label htmlFor={`pomodoro-${field}`} className="text-xs">
        {t(FIELD_LABELS[field])}
      </Label>
      <Input
        id={`pomodoro-${field}`}
        type="number"
        min={min}
        max={max}
        value={settings[field]}
        onChange={(event) => {
          // Read the value before the state update: React nulls `currentTarget` once the
          // handler returns, and the updater would run later, during render.
          const parsed = Number.parseInt(event.target.value, 10);
          if (!Number.isInteger(parsed)) return;
          onChange({ ...settings, [field]: Math.min(max, Math.max(min, parsed)) });
        }}
      />
    </div>
  );
}
