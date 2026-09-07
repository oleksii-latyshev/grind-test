import { useEffect, useMemo, useState } from "react";
import {
  BookOpenCheck,
  CalendarClock,
  Flame,
  GraduationCap,
  History,
  Loader2,
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api, errorMessage } from "@/lib/api";
import {
  MAX_TOPICS,
  MINUTES_PER_TOPIC,
  SESSION_SIZES,
  STAGES,
  formatHours,
  stageOf,
} from "@/lib/study";
import type {
  SessionMode,
  SessionPlan,
  SessionSummary,
  SubjectDetail,
  Topic,
} from "@/lib/types";
import { cn } from "@/lib/utils";

type Pick = "auto" | "manual";

const PACE: Record<SessionMode, { label: string; blurb: string }> = {
  full: {
    label: "Повний",
    blurb:
      "Повний конспект, розгорнута письмова відповідь як на екзамені, 4 питання квізу на тему.",
  },
  sprint: {
    label: "Спринт",
    blurb:
      "Стислий конспект — суть, ключові терміни й типові пастки. Відповідь короткими пунктами, 3 питання квізу на тему.",
  },
};

interface Props {
  detail: SubjectDetail;
  onSessionStart: (plan: SessionPlan) => void;
}

export function StudyTab({ detail, onSessionStart }: Props) {
  const [pace, setPace] = useState<SessionMode>("sprint");
  const [pick, setPick] = useState<Pick>("auto");
  const [size, setSize] = useState(SESSION_SIZES.sprint[1]);
  const [chosen, setChosen] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [starting, setStarting] = useState(false);
  const [unfinished, setUnfinished] = useState<SessionSummary | null>(null);
  const { study } = detail;

  const withNotes = useMemo(() => new Set(detail.knowledge_topic_ids), [detail]);

  useEffect(() => {
    api
      .unfinishedSessions(detail.subject.id)
      .then((sessions) => setUnfinished(sessions[0] ?? null))
      .catch(() => setUnfinished(null));
  }, [detail.subject.id]);

  const ready = study.available > 0;
  const atCap = chosen.length >= MAX_TOPICS[pace];
  const canStart =
    ready && !starting && (pick === "auto" ? study.due_now > 0 : chosen.length > 0);

  // What is left to cover at least once, and what that costs at each pace.
  const remaining = study.available - study.mastered - study.review;
  const estimate = (mode: SessionMode) => formatHours(remaining * MINUTES_PER_TOPIC[mode]);

  function changePace(next: SessionMode) {
    setPace(next);
    setSize(SESSION_SIZES[next][1]);
    setChosen((current) => current.slice(0, MAX_TOPICS[next]));
  }

  async function start() {
    setStarting(true);
    try {
      onSessionStart(
        pick === "manual"
          ? await api.startStudySession(detail.subject.id, chosen.length, pace, chosen)
          : await api.startStudySession(detail.subject.id, size, pace),
      );
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  async function resume(session: SessionSummary) {
    setStarting(true);
    try {
      onSessionStart(await api.resumeStudySession(detail.subject.id, session.id));
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  function toggle(topic: Topic, checked: boolean) {
    setChosen((current) =>
      checked
        ? current.includes(topic.id) || current.length >= MAX_TOPICS[pace]
          ? current
          : [...current, topic.id]
        : current.filter((id) => id !== topic.id),
    );
  }

  return (
    <div className="space-y-6">
      {unfinished ? (
        <Card className="border-chart-3/40">
          <CardContent className="flex flex-wrap items-center justify-between gap-3 py-4">
            <div className="min-w-0 flex-1 space-y-1">
              <p className="flex items-center gap-2 text-sm font-medium">
                <History className="size-4 text-chart-3" />
                Незавершена сесія
              </p>
              <p className="line-clamp-1 text-xs text-muted-foreground">
                {unfinished.topic_titles.join(" · ")}
              </p>
            </div>
            <Button variant="outline" onClick={() => resume(unfinished)} disabled={starting}>
              Продовжити
            </Button>
          </CardContent>
        </Card>
      ) : null}

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

          {remaining > 0 ? (
            <p className="text-xs leading-relaxed text-muted-foreground">
              Залишилось пройти щонайменше раз: <strong>{remaining}</strong> тем — це
              приблизно <strong>{estimate("full")}</strong> у повному режимі або{" "}
              <strong>{estimate("sprint")}</strong> у спринті.
            </p>
          ) : null}
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
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              Темп
            </p>
            <div className="flex flex-wrap gap-2">
              {(["full", "sprint"] as const).map((value) => (
                <Segment key={value} active={pace === value} onClick={() => changePace(value)}>
                  {PACE[value].label}
                </Segment>
              ))}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">
              {PACE[pace].blurb} ≈ {MINUTES_PER_TOPIC[pace]} хв на тему.
            </p>
          </div>

          <div className="flex flex-wrap gap-2">
            <Segment active={pick === "auto"} onClick={() => setPick("auto")}>
              Автоматично
            </Segment>
            <Segment active={pick === "manual"} onClick={() => setPick("manual")}>
              Обрати теми
            </Segment>
          </div>

          {pick === "auto" ? (
            <div className="space-y-2">
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Тем за сесію
              </p>
              <div className="flex flex-wrap gap-2">
                {SESSION_SIZES[pace].map((value) => (
                  <Segment key={value} active={size === value} onClick={() => setSize(value)}>
                    {value}
                  </Segment>
                ))}
              </div>
              <p className="text-xs text-muted-foreground">
                {pace === "sprint"
                  ? "Спершу теми, яких ви ще не бачили, за порядком силабуса — щоб охопити весь предмет."
                  : "Спершу прострочені повторення, далі нові теми за порядком силабуса."}
              </p>
            </div>
          ) : (
            <TopicPicker
              detail={detail}
              withNotes={withNotes}
              chosen={chosen}
              atCap={atCap}
              maxTopics={MAX_TOPICS[pace]}
              query={query}
              onQuery={setQuery}
              onToggle={toggle}
            />
          )}

          <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
            <Button onClick={start} disabled={!canStart}>
              {starting ? <Loader2 className="size-4 animate-spin" /> : null}
              {pick === "manual" && chosen.length > 0
                ? `Почати сесію (${chosen.length})`
                : "Почати сесію"}
            </Button>
            {!ready ? (
              <p className="text-sm text-muted-foreground">
                Спершу згенеруйте конспекти на вкладці «Теми».
              </p>
            ) : pick === "auto" && study.due_now === 0 ? (
              <p className="text-sm text-muted-foreground">
                Усе опрацьовано — або оберіть теми вручну, щоб повторити раніше.
              </p>
            ) : pick === "manual" && chosen.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                Оберіть від однієї до {MAX_TOPICS[pace]} тем.
              </p>
            ) : null}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

function TopicPicker({
  detail,
  withNotes,
  chosen,
  atCap,
  maxTopics,
  query,
  onQuery,
  onToggle,
}: {
  detail: SubjectDetail;
  withNotes: Set<string>;
  chosen: string[];
  atCap: boolean;
  maxTopics: number;
  query: string;
  onQuery: (value: string) => void;
  onToggle: (topic: Topic, checked: boolean) => void;
}) {
  const needle = query.trim().toLowerCase();

  const sections = detail.subject.sections
    .map((section) => ({
      title: section.title,
      topics: section.topics.filter(
        (topic) =>
          withNotes.has(topic.id) &&
          (needle === "" ||
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
          placeholder="Пошук за темою…"
          className="max-w-xs"
        />
        <p className="text-xs text-muted-foreground">
          Обрано {chosen.length} з {maxTopics}
        </p>
      </div>

      <ScrollArea className="h-80 rounded-4xl border border-border">
        {sections.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">Нічого не знайдено.</p>
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
                      "flex cursor-pointer items-start gap-3 px-4 py-2.5 transition-colors hover:bg-muted/50",
                      !checked && atCap && "cursor-not-allowed opacity-50",
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
                        <span className={cn("text-[0.7rem]", STAGES[stage].className)}>
                          {STAGES[stage].label}
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

function Segment({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "rounded-4xl border px-4 py-1.5 text-sm transition-colors",
        active
          ? "border-primary bg-primary text-primary-foreground"
          : "border-input bg-input/30 hover:bg-muted",
      )}
    >
      {children}
    </button>
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
