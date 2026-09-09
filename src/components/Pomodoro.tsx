import { useState } from "react";
import { Coffee, Pause, Play, RotateCcw, Settings2, SkipForward, Timer } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { POMODORO_LIMITS, formatClock, usePomodoro } from "@/lib/pomodoro";
import type { Phase, PomodoroSettings } from "@/lib/pomodoro";
import { cn } from "@/lib/utils";

const PHASES: Record<Phase, { label: string; announce: string; tone: string }> = {
  work: {
    label: "Робота",
    announce: "Перерва закінчилась — до роботи.",
    tone: "text-primary",
  },
  short: {
    label: "Перерва",
    announce: "Час коротко відпочити: встаньте, подивіться у вікно.",
    tone: "text-chart-2",
  },
  long: {
    label: "Довга перерва",
    announce: "Кілька підходів позаду — зробіть довшу перерву.",
    tone: "text-chart-3",
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
  const timer = usePomodoro((next) => toast(PHASES[next].label, { description: PHASES[next].announce }));
  const [open, setOpen] = useState(false);
  const idle = !timer.running && timer.left === timer.total;

  return (
    <div className="flex items-center gap-0.5">
      <Button
        variant="ghost"
        size="sm"
        onClick={timer.running ? timer.pause : timer.start}
        className="gap-1.5 px-2"
        title={timer.running ? "Пауза" : "Запустити таймер"}
      >
        {timer.phase === "work" ? (
          <Timer className={cn("size-4", !idle && PHASES.work.tone)} />
        ) : (
          <Coffee className={cn("size-4", PHASES[timer.phase].tone)} />
        )}
        <span className={cn("font-mono text-xs tabular-nums", !idle && PHASES[timer.phase].tone)}>
          {formatClock(timer.left)}
        </span>
        {timer.running ? <Pause className="size-3" /> : <Play className="size-3" />}
      </Button>

      {idle ? null : (
        <>
          <Button variant="ghost" size="icon" onClick={timer.reset} title="Спочатку">
            <RotateCcw className="size-3.5" />
          </Button>
          <Button variant="ghost" size="icon" onClick={timer.skip} title="Наступна фаза">
            <SkipForward className="size-3.5" />
          </Button>
        </>
      )}

      <Button
        variant="ghost"
        size="icon"
        onClick={() => setOpen(true)}
        title="Налаштування таймера"
      >
        <Settings2 className="size-3.5" />
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Таймер</DialogTitle>
            <DialogDescription>
              Підходів завершено: {timer.round}. Довга перерва — кожні {timer.settings.every}.
            </DialogDescription>
          </DialogHeader>

          <div className="grid grid-cols-2 gap-4">
            <Field label="Робота, хв" field="work" settings={timer.settings} onChange={timer.setSettings} />
            <Field label="Перерва, хв" field="short" settings={timer.settings} onChange={timer.setSettings} />
            <Field label="Довга перерва, хв" field="long" settings={timer.settings} onChange={timer.setSettings} />
            <Field label="Підходів до довгої" field="every" settings={timer.settings} onChange={timer.setSettings} />
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function Field({
  label,
  field,
  settings,
  onChange,
}: {
  label: string;
  field: keyof PomodoroSettings;
  settings: PomodoroSettings;
  onChange: (settings: PomodoroSettings) => void;
}) {
  const [min, max] = POMODORO_LIMITS[field];
  return (
    <div className="space-y-1.5">
      <Label htmlFor={`pomodoro-${field}`} className="text-xs">
        {label}
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
