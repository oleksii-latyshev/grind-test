import { useCallback, useEffect, useState } from "react";
import { ArrowRight, FolderOpen, Loader2, TriangleAlert } from "lucide-react";
import { toast } from "sonner";

import { VaultCard } from "@/components/VaultCard";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Skeleton } from "@/components/ui/skeleton";
import { api, errorMessage } from "@/lib/api";
import type { SubjectOverview, VaultInfo } from "@/lib/types";

interface Props {
  /** Owned by `App`, which decides between this screen and setup. */
  vault: VaultInfo;
  onVaultChange: (vault: VaultInfo) => void;
  onOpen: (subjectId: string) => void;
}

export function SubjectsScreen({ vault, onVaultChange, onOpen }: Props) {
  const [subjects, setSubjects] = useState<SubjectOverview[] | null>(null);
  const [choosing, setChoosing] = useState(false);

  const load = useCallback(async () => {
    if (!vault.readable) {
      // No point asking for subjects we cannot read; the vault card explains why.
      setSubjects([]);
      return;
    }
    try {
      setSubjects(await api.listSubjects());
    } catch (error) {
      toast.error(errorMessage(error));
      setSubjects([]);
    }
  }, [vault.readable, vault.root]);

  useEffect(() => {
    load().catch((error) => toast.error(errorMessage(error)));
  }, [load]);

  async function chooseVault() {
    setChoosing(true);
    try {
      const info = await api.chooseVault();
      onVaultChange(info);
      if (info.readable) {
        toast.success(`Сховище підключено: ${info.subject_count} предметів`);
      } else if (info.error) {
        toast.error(info.error);
      }
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setChoosing(false);
    }
  }

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-8">
      <header className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">Предмети</h1>
        <p className="text-sm text-muted-foreground">
          Оберіть предмет, згенеруйте конспекти за темами і проходьте тести.
        </p>
      </header>

      {!vault.readable ? (
        <VaultCard vault={vault} busy={choosing} onChoose={chooseVault} />
      ) : null}

      {vault.readable && !vault.agy_binary ? (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertTitle>CLI `agy` не знайдено</AlertTitle>
          <AlertDescription>
            Генерація конспектів і тестів не працюватиме. Встановіть Antigravity CLI або
            вкажіть шлях у змінній середовища <code>GRIND_AGY_BIN</code>.
          </AlertDescription>
        </Alert>
      ) : null}

      {subjects === null ? (
        <div className="grid gap-4 sm:grid-cols-2">
          <Skeleton className="h-44 w-full rounded-4xl" />
          <Skeleton className="h-44 w-full rounded-4xl" />
        </div>
      ) : subjects.length === 0 && vault.readable ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-2 py-12 text-center text-sm text-muted-foreground">
            <FolderOpen className="size-6" />
            <p>У сховищі немає жодного силабуса.</p>
            <p className="font-mono text-xs">{vault.root}/syllabus/*.md</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          {subjects.map((subject) => (
            <SubjectCard key={subject.id} subject={subject} onOpen={onOpen} />
          ))}
        </div>
      )}

      {vault.readable ? (
        <footer className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <p className="font-mono text-xs text-muted-foreground">Сховище: {vault.root}</p>
          <Button variant="ghost" size="sm" onClick={chooseVault} disabled={choosing}>
            {choosing ? <Loader2 className="size-4 animate-spin" /> : null}
            Змінити
          </Button>
        </footer>
      ) : null}
    </div>
  );
}

function SubjectCard({
  subject,
  onOpen,
}: {
  subject: SubjectOverview;
  onOpen: (subjectId: string) => void;
}) {
  const coverage = subject.topic_count
    ? Math.round((subject.knowledge_count / subject.topic_count) * 100)
    : 0;

  return (
    <Card className="flex flex-col">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Badge variant="outline" className="font-mono uppercase">
            {subject.id}
          </Badge>
        </CardTitle>
        <CardDescription className="line-clamp-2 leading-snug">{subject.title}</CardDescription>
      </CardHeader>

      <CardContent className="flex flex-1 flex-col justify-end gap-4">
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Конспекти</span>
            <span className="font-mono">
              {subject.knowledge_count}/{subject.topic_count}
            </span>
          </div>
          <Progress value={coverage} />
        </div>

        <div className="flex items-center justify-between">
          <div className="flex gap-4 text-xs text-muted-foreground">
            <span>
              Тестів: <span className="font-medium text-foreground">{subject.quiz_count}</span>
            </span>
            <span>
              Точність:{" "}
              <span className="font-medium text-foreground">{subject.accuracy_percent}%</span>
            </span>
          </div>
          <Button size="sm" onClick={() => onOpen(subject.id)}>
            Відкрити
            <ArrowRight className="size-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
