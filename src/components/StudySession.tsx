import { useEffect, useState } from "react";
import {
  ArrowRight,
  Check,
  ChevronLeft,
  Loader2,
  PenLine,
  Repeat,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import { toast } from "sonner";

import { NoteReader } from "@/components/NoteReader";
import { ReadingSizeControl } from "@/components/ReadingSizeControl";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Textarea } from "@/components/ui/textarea";
import { api, errorMessage } from "@/lib/api";
import { clearDraft, readDraft, writeDraft } from "@/lib/drafts";
import { STAGES, formatDue, scoreTone } from "@/lib/study";
import type { SessionPlan, SessionResult } from "@/lib/types";
import { cn } from "@/lib/utils";

type Step = "read" | "open" | "quiz" | "done";

const STEP_LABELS: Record<Exclude<Step, "done">, string> = {
  read: "Читання",
  open: "Відкриті питання",
  quiz: "Квіз",
};

interface Props {
  plan: SessionPlan;
  onExit: () => void;
  onFinished: () => void;
}

export function StudySession({ plan, onExit, onFinished }: Props) {
  const [step, setStep] = useState<Step>("read");
  const [readIndex, setReadIndex] = useState(0);
  // Restored from the local draft, so re-entering a session brings the typing back.
  const [openAnswers, setOpenAnswers] = useState<Record<string, string>>(() => readDraft(plan.id));
  const [quizSelections, setQuizSelections] = useState<Record<string, number[]>>({});
  const [result, setResult] = useState<SessionResult | null>(null);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    writeDraft(plan.id, openAnswers);
  }, [plan.id, openAnswers]);

  async function advanceReading() {
    const topic = plan.topics[readIndex];
    // Fire and forget: a failed bookkeeping write must not block the session.
    api.markTopicRead(topic.topic.id).catch(() => {});
    if (readIndex + 1 < plan.topics.length) {
      setReadIndex((value) => value + 1);
      window.scrollTo({ top: 0 });
    } else {
      setStep("open");
      window.scrollTo({ top: 0 });
    }
  }

  function chooseOption(questionId: string, option: number, multi: boolean) {
    setQuizSelections((current) => {
      const previous = current[questionId] ?? [];
      if (!multi) return { ...current, [questionId]: [option] };
      const next = previous.includes(option)
        ? previous.filter((value) => value !== option)
        : [...previous, option];
      return { ...current, [questionId]: next };
    });
  }

  async function submit() {
    setSubmitting(true);
    try {
      const outcome = await api.finishStudySession({
        subjectId: plan.subject,
        sessionId: plan.id,
        openAnswers,
        quizSelections,
      });
      setResult(outcome);
      clearDraft(plan.id);
      setStep("done");
      window.scrollTo({ top: 0 });
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setSubmitting(false);
    }
  }

  if (step === "done" && result) {
    return <SessionSummary plan={plan} result={result} openAnswers={openAnswers} onDone={onFinished} />;
  }

  const answeredQuiz = Object.values(quizSelections).filter((s) => s.length > 0).length;
  const sprint = plan.mode === "sprint";

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-8">
      <header className="sticky top-12 z-10 -mx-8 space-y-3 border-b border-border bg-background/95 px-8 py-3 backdrop-blur">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            {(["read", "open", "quiz"] as const).map((value, index) => (
              <span key={value} className="flex items-center gap-2">
                {index > 0 ? <span className="text-border">/</span> : null}
                <span className={cn(step === value && "font-medium text-foreground")}>
                  {STEP_LABELS[value]}
                </span>
              </span>
            ))}
          </div>
          <div className="flex items-center gap-2">
            {step === "read" ? <ReadingSizeControl /> : null}
            <Button variant="ghost" size="sm" onClick={onExit}>
              <X className="size-4" />
              Вийти
            </Button>
          </div>
        </div>
        <Progress value={step === "read" ? ((readIndex + 1) / plan.topics.length) * 60 : step === "open" ? 75 : 90} />
      </header>

      {step === "read" ? (
        <>
          <NoteReader
            note={{
              ...plan.notes[readIndex],
              // A sprint reads the digest; older sessions have no `reading` array.
              body: plan.reading[readIndex] ?? plan.notes[readIndex].body,
            }}
            eyebrow={
              <>
                <Badge variant="outline" className="font-mono text-[0.7rem]">
                  {plan.topics[readIndex].topic.id}
                </Badge>
                <span className="text-xs text-muted-foreground">
                  Тема {readIndex + 1} з {plan.topics.length}
                </span>
                {sprint ? (
                  <Badge variant="secondary" className="gap-1">
                    <Zap className="size-3" />
                    Стисло
                  </Badge>
                ) : null}
                {plan.topics[readIndex].is_review ? (
                  <Badge variant="secondary" className="gap-1">
                    <Repeat className="size-3" />
                    Повторення
                  </Badge>
                ) : null}
              </>
            }
          />
          <div className="mx-auto flex w-full max-w-3xl items-center justify-between border-t border-border pt-6">
            <Button
              variant="outline"
              onClick={() => setReadIndex((value) => value - 1)}
              disabled={readIndex === 0}
            >
              <ChevronLeft className="size-4" />
              Попередня
            </Button>
            <Button onClick={advanceReading}>
              {readIndex + 1 < plan.topics.length ? "Наступна тема" : "До питань"}
              <ArrowRight className="size-4" />
            </Button>
          </div>
        </>
      ) : step === "open" ? (
        <section className="space-y-5">
          <div className="space-y-1">
            <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
              <PenLine className="size-5 text-primary" />
              {sprint ? "Ключові пункти" : "Відкриті питання"}
            </h2>
            <p className="text-sm text-muted-foreground">
              {sprint
                ? "Перелічіть головне короткими пунктами — зв'язний текст не потрібен, оцінюється лише суть."
                : "Відповідайте розгорнуто, як на екзамені."}{" "}
              Розбір побачите наприкінці сесії.
            </p>
          </div>

          {plan.open_questions.map((question, index) => (
            <Card key={question.topic_id}>
              <CardContent className="space-y-3 py-5">
                <Badge variant="outline" className="font-mono text-[0.7rem]">
                  {question.topic_id}
                </Badge>
                <p className="text-sm font-medium leading-snug">
                  {index + 1}. {question.question}
                </p>
                <Textarea
                  rows={sprint ? 5 : 8}
                  value={openAnswers[question.topic_id] ?? ""}
                  onChange={(event) => {
                    // Read the value now: React clears `currentTarget` once the handler
                    // returns, and a functional update runs later, during render — reading
                    // it in there throws and takes the whole tree down.
                    const value = event.target.value;
                    setOpenAnswers((current) => ({
                      ...current,
                      [question.topic_id]: value,
                    }));
                  }}
                  placeholder={sprint ? "Коротко, пунктами…" : "Ваша відповідь…"}
                />
              </CardContent>
            </Card>
          ))}

          <div className="flex justify-between border-t border-border pt-6">
            <Button variant="outline" onClick={() => setStep("read")}>
              <ChevronLeft className="size-4" />
              До конспектів
            </Button>
            <Button onClick={() => setStep("quiz")}>
              До квізу
              <ArrowRight className="size-4" />
            </Button>
          </div>
        </section>
      ) : (
        <section className="space-y-5">
          <div className="space-y-1">
            <h2 className="flex items-center gap-2 text-lg font-semibold tracking-tight">
              <Sparkles className="size-5 text-primary" />
              Міні-квіз
            </h2>
            <p className="text-sm text-muted-foreground">
              Відповіли на {answeredQuiz} з {plan.quiz.length}.
            </p>
          </div>

          {plan.quiz.map((question, index) => {
            const chosen = quizSelections[question.id] ?? [];
            const multi = question.type === "multi";
            return (
              <Card key={question.id}>
                <CardContent className="space-y-3 py-5">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge variant="outline" className="font-mono text-[0.7rem]">
                      {question.topic_id}
                    </Badge>
                    {multi ? <Badge variant="secondary">Декілька правильних</Badge> : null}
                  </div>
                  <p className="text-sm font-medium leading-snug">
                    {index + 1}. {question.question}
                  </p>
                  <ul className="space-y-2">
                    {question.options.map((option, optionIndex) => {
                      const active = chosen.includes(optionIndex);
                      return (
                        <li key={optionIndex}>
                          <button
                            type="button"
                            onClick={() => chooseOption(question.id, optionIndex, multi)}
                            aria-pressed={active}
                            className={cn(
                              "flex w-full items-start gap-3 rounded-4xl border px-4 py-2.5 text-left text-sm transition-colors",
                              active
                                ? "border-primary bg-primary/10"
                                : "border-input hover:bg-muted/60",
                            )}
                          >
                            <span
                              className={cn(
                                "mt-px flex size-5 shrink-0 items-center justify-center border text-[0.7rem] font-medium",
                                multi ? "rounded-md" : "rounded-full",
                                active
                                  ? "border-primary bg-primary text-primary-foreground"
                                  : "border-input text-muted-foreground",
                              )}
                            >
                              {optionIndex + 1}
                            </span>
                            <span className="leading-snug">{option}</span>
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                </CardContent>
              </Card>
            );
          })}

          <div className="flex justify-between border-t border-border pt-6">
            <Button variant="outline" onClick={() => setStep("open")}>
              <ChevronLeft className="size-4" />
              До питань
            </Button>
            <Button onClick={submit} disabled={submitting}>
              {submitting ? <Loader2 className="size-4 animate-spin" /> : null}
              Завершити сесію
            </Button>
          </div>
        </section>
      )}
    </div>
  );
}

function SessionSummary({
  plan,
  result,
  openAnswers,
  onDone,
}: {
  plan: SessionPlan;
  result: SessionResult;
  openAnswers: Record<string, string>;
  onDone: () => void;
}) {
  const answersByQuestion = new Map(result.answers.map((answer) => [answer.question_id, answer]));

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-8">
      <Card>
        <CardContent className="space-y-4 py-6 text-center">
          <p className="text-sm text-muted-foreground">Результат сесії</p>
          <p className={cn("text-5xl font-semibold tabular-nums", scoreTone(result.overall_score))}>
            {result.overall_score}%
          </p>
          <Progress value={result.overall_score} />
          <Button onClick={onDone}>Далі</Button>
        </CardContent>
      </Card>

      <section className="space-y-3">
        <h2 className="text-sm font-semibold tracking-tight">Теми та наступне повторення</h2>
        {result.topics.map((outcome) => (
          <Card key={outcome.topic_id}>
            <CardContent className="flex flex-wrap items-center justify-between gap-3 py-4">
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-snug">{outcome.title}</p>
                <p className="text-xs text-muted-foreground">
                  {STAGES[outcome.stage].label} · рівень {outcome.level} · повторення{" "}
                  {formatDue(outcome.due_at)}
                </p>
              </div>
              <div className="flex items-center gap-3 text-sm">
                {outcome.quiz_total > 0 ? (
                  <span className="text-muted-foreground">
                    квіз {outcome.quiz_correct}/{outcome.quiz_total}
                  </span>
                ) : null}
                <span className={cn("text-lg font-semibold tabular-nums", scoreTone(outcome.score))}>
                  {outcome.score}%
                </span>
              </div>
            </CardContent>
          </Card>
        ))}
      </section>

      {result.gradings.length > 0 ? (
        <section className="space-y-4">
          <h2 className="text-sm font-semibold tracking-tight">Розбір відкритих відповідей</h2>
          {result.gradings.map((grading) => {
            const question = plan.open_questions.find((q) => q.topic_id === grading.topic_id);
            return (
              <Card key={grading.topic_id}>
                <CardContent className="space-y-4 py-5">
                  <div className="flex items-start justify-between gap-3">
                    <p className="flex-1 text-sm font-medium leading-snug">{question?.question}</p>
                    <span className={cn("text-lg font-semibold tabular-nums", scoreTone(grading.score))}>
                      {grading.score}%
                    </span>
                  </div>

                  <p className="text-sm leading-relaxed text-muted-foreground">{grading.verdict}</p>

                  {grading.covered.length > 0 ? (
                    <PointList icon="check" title="Розкрито" items={grading.covered} />
                  ) : null}
                  {grading.missed.length > 0 ? (
                    <PointList icon="cross" title="Пропущено" items={grading.missed} />
                  ) : null}

                  {grading.correction ? (
                    <div className="rounded-4xl border border-border bg-muted/50 p-4">
                      <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                        Доповнення
                      </p>
                      <p className="text-sm leading-relaxed">{grading.correction}</p>
                    </div>
                  ) : null}

                  {openAnswers[grading.topic_id]?.trim() ? (
                    <details className="text-sm">
                      <summary className="cursor-pointer text-xs uppercase tracking-wide text-muted-foreground">
                        Ваша відповідь
                      </summary>
                      <p className="mt-2 whitespace-pre-wrap leading-relaxed text-muted-foreground">
                        {openAnswers[grading.topic_id]}
                      </p>
                    </details>
                  ) : null}
                </CardContent>
              </Card>
            );
          })}
        </section>
      ) : null}

      <section className="space-y-3">
        <h2 className="text-sm font-semibold tracking-tight">Розбір квізу</h2>
        {plan.quiz.map((question, index) => {
          const answer = answersByQuestion.get(question.id);
          const correct = answer?.correct ?? false;
          const selected = answer?.selected ?? [];
          return (
            <Card key={question.id} className={cn(!correct && "border-destructive/40")}>
              <CardContent className="space-y-3 py-4">
                <div className="flex items-start gap-3">
                  <span
                    className={cn(
                      "mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full",
                      correct ? "bg-primary text-primary-foreground" : "bg-destructive text-white",
                    )}
                  >
                    {correct ? <Check className="size-3.5" /> : <X className="size-3.5" />}
                  </span>
                  <p className="text-sm font-medium leading-snug">
                    {index + 1}. {question.question}
                  </p>
                </div>
                <ul className="space-y-1.5 pl-9">
                  {question.options.map((option, optionIndex) => {
                    const isCorrect = question.correct.includes(optionIndex);
                    const isChosen = selected.includes(optionIndex);
                    return (
                      <li
                        key={optionIndex}
                        className={cn(
                          "rounded-2xl px-3 py-1.5 text-sm",
                          isCorrect && "bg-primary/10 font-medium",
                          isChosen && !isCorrect && "bg-destructive/10 line-through",
                        )}
                      >
                        {option}
                      </li>
                    );
                  })}
                </ul>
                {question.explanation ? (
                  <p className="pl-9 text-sm leading-relaxed text-muted-foreground">
                    {question.explanation}
                  </p>
                ) : null}
              </CardContent>
            </Card>
          );
        })}
      </section>
    </div>
  );
}

function PointList({
  icon,
  title,
  items,
}: {
  icon: "check" | "cross";
  title: string;
  items: string[];
}) {
  return (
    <div className="space-y-1.5">
      <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">{title}</p>
      <ul className="space-y-1">
        {items.map((item) => (
          <li key={item} className="flex items-start gap-2 text-sm leading-snug">
            {icon === "check" ? (
              <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
            ) : (
              <X className="mt-0.5 size-3.5 shrink-0 text-destructive" />
            )}
            <span>{item}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
