import { useEffect, useState } from "react";
import { GraduationCap, Moon, Sun } from "lucide-react";

import { QuizScreen } from "@/components/QuizScreen";
import { ResultsScreen } from "@/components/ResultsScreen";
import { SubjectScreen } from "@/components/SubjectScreen";
import { SubjectsScreen } from "@/components/SubjectsScreen";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import type { AttemptResult, Quiz } from "@/lib/types";
import "./App.css";

type View =
  | { name: "subjects" }
  | { name: "subject"; subjectId: string }
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
      <header className="sticky top-0 z-20 flex items-center justify-between border-b border-border bg-background/80 px-6 py-3 backdrop-blur">
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
