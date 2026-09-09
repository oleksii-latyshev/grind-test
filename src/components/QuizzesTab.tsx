import { useState } from "react";
import { Loader2, Play, Wand2 } from "lucide-react";
import { toast } from "sonner";

import { useIntl } from "react-intl";

import { useT } from "@/i18n";
import type { MessageId } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { api, errorMessage } from "@/lib/api";
import type { Difficulty, Quiz, SubjectDetail } from "@/lib/types";
import { cn } from "@/lib/utils";

const COUNTS = [5, 10, 15, 20];
const DIFFICULTIES: { value: Difficulty; label: MessageId }[] = [
  { value: "easy", label: "difficulty.easy" },
  { value: "medium", label: "difficulty.medium" },
  { value: "hard", label: "difficulty.hard" },
  { value: "mixed", label: "difficulty.mixed" },
];

interface Props {
  detail: SubjectDetail;
  onStart: (quiz: Quiz) => void;
  onRefresh: () => void;
}

export function QuizzesTab({ detail, onStart, onRefresh }: Props) {
  const t = useT();
  const intl = useIntl();
  const [count, setCount] = useState(10);
  const [difficulty, setDifficulty] = useState<Difficulty>("mixed");
  const [generating, setGenerating] = useState(false);
  const [opening, setOpening] = useState<string | null>(null);

  const ready = detail.knowledge_topic_ids.length > 0;

  async function generate() {
    setGenerating(true);
    try {
      const quiz = await api.generateQuiz({
        subject: detail.subject.id,
        topic_ids: null,
        question_count: count,
        difficulty,
      });
      onRefresh();
      onStart(quiz);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setGenerating(false);
    }
  }

  async function open(quizId: string) {
    setOpening(quizId);
    try {
      onStart(await api.getQuiz(detail.subject.id, quizId));
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setOpening(null);
    }
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>{t("quizzes.new")}</CardTitle>
          <CardDescription>{t("quizzes.new.blurb")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Field label={t("quizzes.count")}>
            {COUNTS.map((value) => (
              <Choice key={value} active={count === value} onClick={() => setCount(value)}>
                {value}
              </Choice>
            ))}
          </Field>

          <Field label={t("quizzes.difficulty")}>
            {DIFFICULTIES.map((option) => (
              <Choice
                key={option.value}
                active={difficulty === option.value}
                onClick={() => setDifficulty(option.value)}
              >
                {t(option.label)}
              </Choice>
            ))}
          </Field>

          <div className="flex items-center gap-3 pt-1">
            <Button onClick={generate} disabled={generating || !ready}>
              {generating ? <Loader2 className="size-4 animate-spin" /> : <Wand2 className="size-4" />}
              {t("quizzes.generate")}
            </Button>
            {!ready ? (
              <p className="text-sm text-muted-foreground">{t("quizzes.needNotes")}</p>
            ) : null}
          </div>
        </CardContent>
      </Card>

      <section className="space-y-3">
        <h3 className="text-sm font-semibold tracking-tight">{t("quizzes.saved")}</h3>
        {detail.quizzes.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("quizzes.none")}</p>
        ) : (
          <ul className="divide-y divide-border rounded-4xl border border-border">
            {detail.quizzes.map((quiz) => (
              <li
                key={quiz.id}
                className="flex items-center justify-between gap-4 px-4 py-3 first:rounded-t-4xl last:rounded-b-4xl"
              >
                <div className="min-w-0">
                  <p className="truncate text-sm font-medium">{quiz.title}</p>
                  <p className="text-xs text-muted-foreground">
                    {intl.formatDate(quiz.created_at, { dateStyle: "medium", timeStyle: "short" })}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <Badge variant="secondary">{t("quizzes.questions", { count: quiz.question_count })}</Badge>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => open(quiz.id)}
                    disabled={opening !== null}
                  >
                    {opening === quiz.id ? (
                      <Loader2 className="size-4 animate-spin" />
                    ) : (
                      <Play className="size-4" />
                    )}
                    {t("quizzes.take")}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="space-y-2">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{label}</p>
      <div className="flex flex-wrap gap-2">{children}</div>
    </div>
  );
}

function Choice({
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
