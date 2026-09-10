import { useEffect, useMemo, useState } from "react";
import {
  BookOpen,
  BookOpenCheck,
  CalendarClock,
  Flame,
  GraduationCap,
  History,
  Loader2,
} from "lucide-react";
import { toast } from "sonner";

import { useT } from "@/i18n";
import type { MessageId, Translate } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api, errorMessage } from "@/lib/api";
import { usePreference, useSessionSettings } from "@/lib/prefs";
import type { TopicPick } from "@/lib/prefs";
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

const PACE: Record<SessionMode, { label: MessageId; blurb: MessageId }> = {
  full: { label: "study.pace.full", blurb: "study.pace.full.blurb" },
  balanced: { label: "study.pace.balanced", blurb: "study.pace.balanced.blurb" },
  sprint: { label: "study.pace.sprint", blurb: "study.pace.sprint.blurb" },
};

const PICK: Record<TopicPick, { label: MessageId; blurb: MessageId | null }> = {
  auto: { label: "study.pick.auto", blurb: "study.pick.auto.blurb" },
  spread: { label: "study.pick.spread", blurb: "study.pick.spread.blurb" },
  manual: { label: "study.pick.manual", blurb: null },
};

/** Multiples of three, so every batch still draws from the beginning, middle and end. */
const READING_SIZES = [3, 6, 9];

const isReadingSize = (value: unknown): value is number =>
  typeof value === "number" && READING_SIZES.includes(value);

interface Props {
  detail: SubjectDetail;
  onSessionStart: (plan: SessionPlan) => void;
  onReadingStart: (topics: Topic[]) => void;
}

export function StudyTab({ detail, onSessionStart, onReadingStart }: Props) {
  // Remembered per machine: these three are the same on almost every visit, and re-picking
  // them each time is the friction between deciding to study and studying.
  const t = useT();
  const [settings, setSettings] = useSessionSettings();
  const { pace, pick, size } = settings;
  const [chosen, setChosen] = useState<string[]>([]);
  const [query, setQuery] = useState("");
  const [starting, setStarting] = useState(false);
  const [unfinished, setUnfinished] = useState<SessionSummary | null>(null);
  const [readingSize, setReadingSize] = usePreference("grind:reading-size", 3, isReadingSize);
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
    ready &&
    !starting &&
    (pick === "manual" ? chosen.length > 0 : pick === "spread" || study.due_now + study.new > 0);

  // What is left to cover at least once, and what that costs at each pace.
  const remaining = study.new;
  // The headline is coverage — what the student has started. The ladder percentage moves in
  // steps of one level out of six per topic, so on its own it reads as "nothing done".
  const started = study.available - study.new;
  const coverage = study.available > 0 ? Math.round((started / study.available) * 100) : 0;
  const estimate = (mode: SessionMode) => formatHours(t, remaining * MINUTES_PER_TOPIC[mode]);

  function changePace(next: SessionMode) {
    // The size steps differ per pace, so the size moves with it rather than being carried
    // over into a range where it no longer appears as an option.
    setSettings({ ...settings, pace: next, size: SESSION_SIZES[next][1] });
    setChosen((current) => current.slice(0, MAX_TOPICS[next]));
  }

  async function start() {
    setStarting(true);
    try {
      // Disk only: this returns before any model call, so the reader opens at once and the
      // question generation runs behind the first note.
      onSessionStart(
        await api.planStudySession({
          subjectId: detail.subject.id,
          size: pick === "manual" ? chosen.length : size,
          mode: pace,
          selection: pick === "spread" ? "spread" : "scheduled",
          topicIds: pick === "manual" ? chosen : undefined,
        }),
      );
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setStarting(false);
    }
  }

  async function startReading() {
    setStarting(true);
    try {
      const topics = await api.planReading(detail.subject.id, readingSize);
      if (topics.length === 0) toast.info(t("reading.allRead"));
      else onReadingStart(topics);
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
                {t("study.unfinished")}
              </p>
              <p className="line-clamp-1 text-xs text-muted-foreground">
                {unfinished.topic_titles.join(" · ")}
              </p>
            </div>
            <Button variant="outline" onClick={() => resume(unfinished)} disabled={starting}>
              {t("common.continue")}
            </Button>
          </CardContent>
        </Card>
      ) : null}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <GraduationCap className="size-5 text-primary" />
            {t("study.progress.title")}
          </CardTitle>
          <CardDescription>{t("study.progress.blurb")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <div className="flex items-baseline justify-between">
              <span className="text-3xl font-semibold tabular-nums">{coverage}%</span>
              <span className="text-xs text-muted-foreground">
                {t("study.started", { started, total: study.available })}
              </span>
            </div>
            <Progress value={coverage} />
            <p className="text-xs text-muted-foreground">
              {t("study.retained", { percent: study.progress_percent })}
            </p>
          </div>

          <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <Tally label={t("stage.new")} value={study.new} />
            <Tally label={t("stage.learning")} value={study.learning} tone="text-chart-2" />
            <Tally label={t("stage.review")} value={study.review} tone="text-chart-3" />
            <Tally label={t("stage.mastered")} value={study.mastered} tone="text-primary" />
          </div>

          <div className="flex flex-wrap gap-4 border-t border-border pt-4 text-sm">
            <span className="flex items-center gap-2">
              <CalendarClock className="size-4 text-muted-foreground" />
              {t("study.dueNow")} <strong className="tabular-nums">{study.due_now}</strong>
            </span>
            <span className="flex items-center gap-2">
              <Flame className="size-4 text-muted-foreground" />
              {t("study.today")} <strong className="tabular-nums">{study.studied_today}</strong>
            </span>
          </div>

          {remaining > 0 ? (
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t("study.remaining", {
                count: remaining,
                full: estimate("full"),
                balanced: estimate("balanced"),
                sprint: estimate("sprint"),
              })}
            </p>
          ) : null}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <BookOpen className="size-5 text-primary" />
            {t("reading.title")}
          </CardTitle>
          <CardDescription>{t("reading.blurb")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("study.size")}
            </p>
            <div className="flex flex-wrap gap-2">
              {READING_SIZES.map((value) => (
                <Segment key={value} active={readingSize === value} onClick={() => setReadingSize(value)}>
                  {value}
                </Segment>
              ))}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
            <Button onClick={startReading} disabled={!ready || starting || study.new === 0}>
              {starting ? <Loader2 className="size-4 animate-spin" /> : null}
              {t("reading.start")}
            </Button>
            <p className="text-sm text-muted-foreground">
              {!ready
                ? t("study.needNotes")
                : study.new === 0
                  ? t("reading.allRead")
                  : t("reading.left", { count: study.new })}
            </p>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <BookOpenCheck className="size-5 text-primary" />
            {t("study.new.title")}
          </CardTitle>
          <CardDescription>{t("study.new.blurb")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("study.pace")}
            </p>
            <div className="flex flex-wrap gap-2">
              {(["full", "balanced", "sprint"] as const).map((value) => (
                <Segment key={value} active={pace === value} onClick={() => changePace(value)}>
                  {t(PACE[value].label)}
                </Segment>
              ))}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">
              {t(PACE[pace].blurb)} {t("study.pace.perTopic", { minutes: MINUTES_PER_TOPIC[pace] })}
            </p>
          </div>

          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("study.pick")}
            </p>
            <div className="flex flex-wrap gap-2">
              {(["auto", "spread", "manual"] as const).map((value) => (
                <Segment key={value} active={pick === value} onClick={() => setSettings({ ...settings, pick: value })}>
                  {t(PICK[value].label)}
                </Segment>
              ))}
            </div>
          </div>

          {pick !== "manual" ? (
            <div className="space-y-2">
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                {t("study.size")}
              </p>
              <div className="flex flex-wrap gap-2">
                {SESSION_SIZES[pace].map((value) => (
                  <Segment key={value} active={size === value} onClick={() => setSettings({ ...settings, size: value })}>
                    {value}
                  </Segment>
                ))}
              </div>
              <p className="text-xs leading-relaxed text-muted-foreground">
                {pick === "spread"
                  ? t("study.pick.spread.blurb")
                  : pace === "sprint"
                    ? t("study.pick.sprint.blurb")
                    : t("study.pick.auto.blurb")}
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
              t={t}
            />
          )}

          <div className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
            <Button onClick={start} disabled={!canStart}>
              {starting ? <Loader2 className="size-4 animate-spin" /> : null}
              {pick === "manual" && chosen.length > 0
                ? t("study.start.count", { count: chosen.length })
                : t("study.start")}
            </Button>
            {!ready ? (
              <p className="text-sm text-muted-foreground">{t("study.needNotes")}</p>
            ) : pick === "auto" && study.due_now + study.new === 0 ? (
              <p className="text-sm text-muted-foreground">{t("study.allDone")}</p>
            ) : pick === "manual" && chosen.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t("study.pickSome", { max: MAX_TOPICS[pace] })}
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
  t,
}: {
  detail: SubjectDetail;
  withNotes: Set<string>;
  chosen: string[];
  atCap: boolean;
  maxTopics: number;
  query: string;
  onQuery: (value: string) => void;
  onToggle: (topic: Topic, checked: boolean) => void;
  t: Translate;
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
          placeholder={t("study.search")}
          className="max-w-xs"
        />
        <p className="text-xs text-muted-foreground">
          {t("study.chosen", { count: chosen.length, max: maxTopics })}
        </p>
      </div>

      <ScrollArea className="h-80 rounded-4xl border border-border">
        {sections.length === 0 ? (
          <p className="p-4 text-sm text-muted-foreground">{t("study.nothingFound")}</p>
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
