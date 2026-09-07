import { useCallback, useEffect, useState } from "react";
import { ArrowLeft, Loader2 } from "lucide-react";

import { ProgressTab } from "@/components/ProgressTab";
import { QuizzesTab } from "@/components/QuizzesTab";
import { TopicsTab } from "@/components/TopicsTab";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api, errorMessage } from "@/lib/api";
import type { Quiz, SubjectDetail } from "@/lib/types";

interface Props {
  subjectId: string;
  onBack: () => void;
  onStartQuiz: (quiz: Quiz) => void;
}

export function SubjectScreen({ subjectId, onBack, onStartQuiz }: Props) {
  const [detail, setDetail] = useState<SubjectDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    api
      .getSubject(subjectId)
      .then(setDetail)
      .catch((err) => setError(errorMessage(err)));
  }, [subjectId]);

  useEffect(load, [load]);

  if (error) {
    return (
      <div className="mx-auto max-w-5xl space-y-4 p-8">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ArrowLeft className="size-4" />
          Назад
        </Button>
        <p className="text-sm text-destructive">{error}</p>
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="flex items-center justify-center gap-2 p-16 text-sm text-muted-foreground">
        <Loader2 className="size-4 animate-spin" />
        Завантаження…
      </div>
    );
  }

  return (
    <div className="mx-auto w-full max-w-5xl space-y-6 p-8">
      <div className="space-y-3">
        <Button variant="ghost" size="sm" onClick={onBack} className="-ml-2">
          <ArrowLeft className="size-4" />
          Усі предмети
        </Button>
        <div className="flex items-start gap-3">
          <Badge variant="outline" className="mt-1 font-mono uppercase">
            {detail.subject.id}
          </Badge>
          <h1 className="text-2xl font-semibold leading-tight tracking-tight">
            {detail.subject.title}
          </h1>
        </div>
      </div>

      <Tabs defaultValue="topics">
        <TabsList>
          <TabsTrigger value="topics">Теми</TabsTrigger>
          <TabsTrigger value="quizzes">Тести</TabsTrigger>
          <TabsTrigger value="progress">Прогрес</TabsTrigger>
        </TabsList>

        <TabsContent value="topics" className="pt-6">
          <TopicsTab detail={detail} onRefresh={load} />
        </TabsContent>
        <TabsContent value="quizzes" className="pt-6">
          <QuizzesTab detail={detail} onStart={onStartQuiz} onRefresh={load} />
        </TabsContent>
        <TabsContent value="progress" className="pt-6">
          <ProgressTab detail={detail} />
        </TabsContent>
      </Tabs>
    </div>
  );
}
