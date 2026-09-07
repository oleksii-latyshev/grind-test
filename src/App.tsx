import { useEffect, useState } from "react";
import { GraduationCap, Moon, Sun } from "lucide-react";

import { QuizScreen } from "@/components/QuizScreen";
import { ReaderScreen } from "@/components/ReaderScreen";
import { ResultsScreen } from "@/components/ResultsScreen";
import { StudySession } from "@/components/StudySession";
import { SubjectScreen } from "@/components/SubjectScreen";
import { SubjectsScreen } from "@/components/SubjectsScreen";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import type { AttemptResult, Quiz, SessionPlan, Topic } from "@/lib/types";
import "./App.css";

type View =
  | { name: "subjects" }
  | { name: "subject"; subjectId: string }
  | { name: "reader"; topic: Topic }
  | { name: "session"; plan: SessionPlan }
  | { name: "quiz"; quiz: Quiz }
  | { name: "results"; quiz: Quiz; result: AttemptResult };

function App() {
  const [view, setView] = useState<View>({ name: "subjects" });
  const [dark, setDark] = useState(
    () =>
      localStorage.getItem("theme") === "dark" ||
      (localStorage.getItem("theme") === null &&
        window.matchMedia("(prefers-color-scheme: dark)").matches),
  );

  useEffect(() => {
    document.documentElement.classList.toggle("dark", dark);
    localStorage.setItem("theme", dark ? "dark" : "light");
  }, [dark]);

  return (
    <div className="min-h-screen bg-background">
      <header className="sticky top-0 z-20 flex h-12 items-center justify-between border-b border-border bg-background/80 px-6 backdrop-blur">
        <button
          type="button"
          className="flex items-center gap-2 text-sm font-semibold tracking-tight"
          onClick={() => setView({ name: "subjects" })}
        >
          <GraduationCap className="size-5 text-primary" />
          grind-test
        </button>
        <Button variant="ghost" size="icon" onClick={() => setDark((value) => !value)}>
          {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
          <span className="sr-only">Змінити тему</span>
        </Button>
      </header>

      <main>
        {view.name === "subjects" ? (
          <SubjectsScreen onOpen={(subjectId) => setView({ name: "subject", subjectId })} />
        ) : view.name === "subject" ? (
          <SubjectScreen
            subjectId={view.subjectId}
            onBack={() => setView({ name: "subjects" })}
            onStartQuiz={(quiz) => setView({ name: "quiz", quiz })}
            onStartSession={(plan) => setView({ name: "session", plan })}
            onOpenTopic={(topic) => setView({ name: "reader", topic })}
          />
        ) : view.name === "reader" ? (
          <ReaderScreen
            topic={view.topic}
            onBack={() => setView({ name: "subject", subjectId: view.topic.subject })}
          />
        ) : view.name === "session" ? (
          <StudySession
            plan={view.plan}
            onExit={() => setView({ name: "subject", subjectId: view.plan.subject })}
            onFinished={() => setView({ name: "subject", subjectId: view.plan.subject })}
          />
        ) : view.name === "quiz" ? (
          <QuizScreen
            quiz={view.quiz}
            onExit={() => setView({ name: "subject", subjectId: view.quiz.subject })}
            onFinish={(result, quiz) => setView({ name: "results", quiz, result })}
          />
        ) : (
          <ResultsScreen
            quiz={view.quiz}
            result={view.result}
            onBackToSubject={() => setView({ name: "subject", subjectId: view.quiz.subject })}
          />
        )}
      </main>

      <Toaster />
    </div>
  );
}

export default App;
