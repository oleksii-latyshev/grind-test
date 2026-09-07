import { useEffect, useState } from "react";

import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { api } from "@/lib/api";
import type { Attempt, SubjectDetail } from "@/lib/types";

export function ProgressTab({ detail }: { detail: SubjectDetail }) {
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const { stats } = detail;

  useEffect(() => {
    api.listAttempts(detail.subject.id).then(setAttempts).catch(() => setAttempts([]));
  }, [detail.subject.id]);

  return (
    <div className="space-y-6">
      <div className="grid gap-4 sm:grid-cols-3">
        <Stat label="Точність" value={`${stats.accuracy_percent}%`}>
          <Progress value={stats.accuracy_percent} />
        </Stat>
        <Stat
          label="Відповідей"
          value={`${stats.questions_correct}/${stats.questions_answered}`}
        />
        <Stat
          label="Тем опрацьовано"
          value={`${stats.topics_practised}/${stats.topics_total}`}
        >
          <Progress
            value={stats.topics_total ? (stats.topics_practised / stats.topics_total) * 100 : 0}
          />
        </Stat>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Найслабші теми</CardTitle>
          <CardDescription>
            Саме ці теми частіше потраплятимуть у наступні тести.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {stats.weakest.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              Пройдіть перший тест, щоб побачити статистику.
            </p>
          ) : (
            <ul className="space-y-2">
              {stats.weakest.map((topic) => (
                <li key={topic.topic_id} className="flex items-start gap-3">
                  <Badge
                    variant={topic.accuracy_percent < 50 ? "destructive" : "secondary"}
                    className="shrink-0 font-mono"
                  >
                    {topic.accuracy_percent}%
                  </Badge>
                  <span className="flex-1 text-sm leading-snug">{topic.title}</span>
                  <span className="shrink-0 font-mono text-xs text-muted-foreground">
                    {topic.correct}/{topic.seen}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Історія спроб</CardTitle>
        </CardHeader>
        <CardContent>
          {attempts.length === 0 ? (
            <p className="text-sm text-muted-foreground">Ще немає жодної спроби.</p>
          ) : (
            <ul className="divide-y divide-border">
              {attempts.slice(0, 15).map((attempt) => {
                const correct = attempt.answers.filter((a) => a.correct).length;
                const percent = attempt.answers.length
                  ? Math.round((correct / attempt.answers.length) * 100)
                  : 0;
                return (
                  <li key={attempt.id} className="flex items-center justify-between gap-4 py-2.5">
                    <div className="min-w-0">
                      <p className="truncate text-sm">{attempt.quiz_title}</p>
                      <p className="text-xs text-muted-foreground">
                        {new Date(attempt.finished_at).toLocaleString("uk-UA")}
                      </p>
                    </div>
                    <Badge variant={percent >= 70 ? "default" : "secondary"} className="font-mono">
                      {correct}/{attempt.answers.length} · {percent}%
                    </Badge>
                  </li>
                );
              })}
            </ul>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function Stat({
  label,
  value,
  children,
}: {
  label: string;
  value: string;
  children?: React.ReactNode;
}) {
  return (
    <Card>
      <CardContent className="space-y-2 py-5">
        <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{label}</p>
        <p className="text-2xl font-semibold tabular-nums">{value}</p>
        {children}
      </CardContent>
    </Card>
  );
}
