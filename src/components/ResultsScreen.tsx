import { Check, RotateCcw, X } from "lucide-react";

import { useT } from "@/i18n";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import type { AttemptResult, Quiz } from "@/lib/types";
import { cn } from "@/lib/utils";

interface Props {
  quiz: Quiz;
  result: AttemptResult;
  onBackToSubject: () => void;
}

export function ResultsScreen({ quiz, result, onBackToSubject }: Props) {
  const t = useT();
  const byQuestion = new Map(result.attempt.answers.map((answer) => [answer.question_id, answer]));

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 p-8">
      <Card>
        <CardContent className="space-y-4 py-6 text-center">
          <p className="text-sm text-muted-foreground">{t("results.title")}</p>
          <p className="text-5xl font-semibold tabular-nums">{result.score_percent}%</p>
          <p className="text-sm text-muted-foreground">
            {t("results.correctOf", { correct: result.correct, total: result.total })}
          </p>
          <Progress value={result.score_percent} />
          <div className="flex justify-center gap-2 pt-2">
            <Button onClick={onBackToSubject}>
              <RotateCcw className="size-4" />
              {t("session.toSubject")}
            </Button>
          </div>
        </CardContent>
      </Card>

      <section className="space-y-4">
        <h2 className="text-sm font-semibold tracking-tight">{t("results.review")}</h2>
        {quiz.questions.map((question, index) => {
          const answer = byQuestion.get(question.id);
          const selected = answer?.selected ?? [];
          const correct = answer?.correct ?? false;

          return (
            <Card key={question.id} className={cn(!correct && "border-destructive/40")}>
              <CardContent className="space-y-4 py-5">
                <div className="flex items-start gap-3">
                  <span
                    className={cn(
                      "mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full",
                      correct
                        ? "bg-primary text-primary-foreground"
                        : "bg-destructive text-white",
                    )}
                  >
                    {correct ? <Check className="size-3.5" /> : <X className="size-3.5" />}
                  </span>
                  <div className="space-y-1">
                    <p className="text-sm font-medium leading-snug">
                      {index + 1}. {question.question}
                    </p>
                    <Badge variant="outline" className="font-mono text-[0.7rem]">
                      {question.topic_id}
                    </Badge>
                  </div>
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
                        {isChosen ? (
                          <span className="ml-2 text-xs text-muted-foreground">{t("results.yourChoice")}</span>
                        ) : null}
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
