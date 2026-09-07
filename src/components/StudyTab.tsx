import { useState } from "react";
import { BookOpenCheck, CalendarClock, Flame, GraduationCap, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { api, errorMessage } from "@/lib/api";
import type { SessionPlan, SubjectDetail } from "@/lib/types";
import { cn } from "@/lib/utils";

const SIZES = [2, 3, 5];

interface Props {
  detail: SubjectDetail;
  onSessionStart: (plan: SessionPlan) => void;
}

export function StudyTab({ detail, onSessionStart }: Props) {
  const [size, setSize] = useState(3);
  const [starting, setStarting] = useState(false);
  const { study } = detail;

  const ready = study.available > 0;
  const nothingDue = ready && study.due_now === 0;

  async function start() {
    setStarting(true);
    try {
      onSessionStart(await api.startStudySession(detail.subject.id, size));
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GraduationCap className="size-5 text-primary" />
            Прогрес вивчення
          </CardTitle>
          <CardDescription>
            Тема піднімається сходинкою вгору після кожної успішної сесії й повертається на
            повторення через 1 → 2 → 4 → 7 → 14 → 30 днів.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <div className="flex items-baseline justify-between">
              <span className="text-3xl font-semibold tabular-nums">
                {study.progress_percent}%
              </span>
              <span className="text-xs text-muted-foreground">
                {study.available} тем із конспектами
              </span>
            </div>
            <Progress value={study.progress_percent} />
          </div>

          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Tally label="Не почато" value={study.new} />
            <Tally label="Вивчається" value={study.learning} tone="text-chart-2" />
            <Tally label="Повторення" value={study.review} tone="text-chart-3" />
            <Tally label="Засвоєно" value={study.mastered} tone="text-primary" />
          </div>

          <div className="flex flex-wrap gap-4 border-t border-border pt-4 text-sm">
            <span className="flex items-center gap-2">
              <CalendarClock className="size-4 text-muted-foreground" />
              До опрацювання зараз: <strong className="tabular-nums">{study.due_now}</strong>
            </span>
            <span className="flex items-center gap-2">
              <Flame className="size-4 text-muted-foreground" />
              Сьогодні опрацьовано: <strong className="tabular-nums">{study.studied_today}</strong>
            </span>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <BookOpenCheck className="size-5 text-primary" />
            Нова сесія
          </CardTitle>
          <CardDescription>
            Читаєте конспекти, відповідаєте на відкриті питання екзаменаційного формату, потім
            закріплюєте міні-квізом. Відповіді перевіряє розумна модель.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Тем за сесію
            </p>
            <div className="flex flex-wrap gap-2">
              {SIZES.map((value) => (
                <button
                  key={value}
                  type="button"
                  onClick={() => setSize(value)}
                  aria-pressed={size === value}
                  className={cn(
                    "rounded-4xl border px-4 py-1.5 text-sm transition-colors",
                    size === value
                      ? "border-primary bg-primary text-primary-foreground"
                      : "border-input bg-input/30 hover:bg-muted",
                  )}
                >
                  {value}
                </button>
              ))}
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3 pt-1">
            <Button onClick={start} disabled={starting || !ready || nothingDue}>
              {starting ? <Loader2 className="size-4 animate-spin" /> : null}
              Почати сесію
            </Button>
            {!ready ? (
              <p className="text-sm text-muted-foreground">
                Спершу згенеруйте конспекти на вкладці «Теми».
              </p>
            ) : nothingDue ? (
              <p className="text-sm text-muted-foreground">
                Усе опрацьовано — наступні повторення заплановані на потім.
              </p>
            ) : null}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function Tally({ label, value, tone }: { label: string; value: number; tone?: string }) {
  return (
    <div className="rounded-4xl border border-border px-4 py-3">
      <p className={cn("text-xl font-semibold tabular-nums", tone)}>{value}</p>
      <p className="text-xs text-muted-foreground">{label}</p>
    </div>
  );
}
