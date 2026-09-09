import { useEffect, useState } from "react";
import { CircleQuestionMark, GraduationCap, Loader2, Moon, Settings, Sun } from "lucide-react";
import { toast } from "sonner";

import { ErrorBoundary } from "@/components/ErrorBoundary";
import { GuideScreen } from "@/components/GuideScreen";
import { Pomodoro } from "@/components/Pomodoro";
import { QuizScreen } from "@/components/QuizScreen";
import { ReaderScreen } from "@/components/ReaderScreen";
import { ResultsScreen } from "@/components/ResultsScreen";
import { SettingsScreen } from "@/components/SettingsScreen";
import { SetupScreen } from "@/components/SetupScreen";
import { StudySession } from "@/components/StudySession";
import { SubjectScreen } from "@/components/SubjectScreen";
import { SubjectsScreen } from "@/components/SubjectsScreen";
import { Button } from "@/components/ui/button";
import { Toaster } from "@/components/ui/sonner";
import { api, errorMessage } from "@/lib/api";
import { usePreference } from "@/lib/prefs";
import type { AttemptResult, Quiz, SessionPlan, Topic, VaultInfo } from "@/lib/types";
import "./App.css";

type View =
  | { name: "subjects" }
  | { name: "settings" }
  | { name: "guide" }
  | { name: "subject"; subjectId: string }
  | { name: "reader"; topic: Topic }
  | { name: "session"; plan: SessionPlan }
  | { name: "quiz"; quiz: Quiz }
  | { name: "results"; quiz: Quiz; result: AttemptResult };

function App() {
  const [view, setView] = useState<View>({ name: "subjects" });
  // Owned here rather than in the subjects screen: whether setup has happened decides which
  // screen exists at all, and nothing may read the vault until it has.
  const [vault, setVault] = useState<VaultInfo | null>(null);
  // The guide opens itself once, right after setup; from then on it is a header button.
  const [guideSeen, setGuideSeen] = usePreference<boolean>(
    "grind:guide-seen",
    false,
    (value): value is boolean => typeof value === "boolean",
  );
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

  useEffect(() => {
    api
      .vaultInfo()
      .then(setVault)
      .catch((error) => toast.error(errorMessage(error)));
  }, []);

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
        <div className="flex items-center gap-1">
          {/* In the header rather than inside a session: the clock has to keep running while
              you browse topics or read a note outside one. */}
          {vault?.configured ? <Pomodoro /> : null}
          {vault?.configured ? (
            <>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setView({ name: "guide" })}
                title="Як це працює"
              >
                <CircleQuestionMark className="size-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setView({ name: "settings" })}
                title="Моделі"
              >
                <Settings className="size-4" />
              </Button>
            </>
          ) : null}
          <Button variant="ghost" size="icon" onClick={() => setDark((value) => !value)}>
            {dark ? <Sun className="size-4" /> : <Moon className="size-4" />}
            <span className="sr-only">Змінити тему</span>
          </Button>
        </div>
      </header>

      <main>
        <ErrorBoundary onReset={() => setView({ name: "subjects" })}>
        {vault === null ? (
          <div className="flex items-center justify-center gap-2 p-16 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Завантаження…
          </div>
        ) : !vault.configured ? (
          <SetupScreen vault={vault} onReady={setVault} />
        ) : !guideSeen || view.name === "guide" ? (
          <GuideScreen
            firstRun={!guideSeen}
            onDone={() => {
              setGuideSeen(true);
              setView({ name: "subjects" });
            }}
          />
        ) : view.name === "settings" ? (
          <SettingsScreen onBack={() => setView({ name: "subjects" })} />
        ) : view.name === "subjects" ? (
          <SubjectsScreen
            vault={vault}
            onVaultChange={setVault}
            onOpen={(subjectId) => setView({ name: "subject", subjectId })}
          />
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
            // A new session must start from scratch — step, reading position, drafts and
            // the result of the one just finished all belong to the old id.
            key={view.plan.id}
            plan={view.plan}
            onExit={() => setView({ name: "subject", subjectId: view.plan.subject })}
            onFinished={() => setView({ name: "subject", subjectId: view.plan.subject })}
            onContinue={(plan) => setView({ name: "session", plan })}
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
        </ErrorBoundary>
      </main>

      <Toaster />
    </div>
  );
}

export default App;
