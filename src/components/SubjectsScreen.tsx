import { ArrowRight, FolderOpen, Loader2, TriangleAlert } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Skeleton } from '@/components/ui/skeleton';
import { VaultCard } from '@/components/VaultCard';
import type { Translate } from '@/i18n';
import { useT } from '@/i18n';
import { api, errorMessage } from '@/lib/api';
import type { SubjectOverview, VaultInfo } from '@/lib/types';

interface Props {
  /** Owned by `App`, which decides between this screen and setup. */
  vault: VaultInfo;
  onVaultChange: (vault: VaultInfo) => void;
  onOpen: (subjectId: string) => void;
}

export function SubjectsScreen({ vault, onVaultChange, onOpen }: Props) {
  const t = useT();
  const [subjects, setSubjects] = useState<SubjectOverview[] | null>(null);
  const [choosing, setChoosing] = useState(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: a different vault folder must reload the list.
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
        toast.success(t('vault.connected', { count: info.subject_count }));
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
        <h1 className="text-2xl font-semibold tracking-tight">{t('subjects.title')}</h1>
        <p className="text-sm text-muted-foreground">{t('subjects.subtitle')}</p>
      </header>

      {!vault.readable ? <VaultCard vault={vault} busy={choosing} onChoose={chooseVault} /> : null}

      {vault.readable && !vault.agy_binary ? (
        <Alert variant="destructive">
          <TriangleAlert />
          <AlertTitle>{t('agy.missing.title')}</AlertTitle>
          <AlertDescription>{t('agy.missing.body')}</AlertDescription>
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
            <p>{t('subjects.empty')}</p>
            <p className="font-mono text-xs">{vault.root}/syllabus/*.md</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2">
          {subjects.map((subject) => (
            <SubjectCard key={subject.id} subject={subject} onOpen={onOpen} t={t} />
          ))}
        </div>
      )}

      {vault.readable ? (
        <footer className="flex flex-wrap items-center gap-3 border-t border-border pt-4">
          <p className="font-mono text-xs text-muted-foreground">
            {t('vault.label', { root: vault.root })}
          </p>
          <Button variant="ghost" size="sm" onClick={chooseVault} disabled={choosing}>
            {choosing ? <Loader2 className="size-4 animate-spin" /> : null}
            {t('vault.change')}
          </Button>
        </footer>
      ) : null}
    </div>
  );
}

function SubjectCard({
  subject,
  onOpen,
  t,
}: {
  subject: SubjectOverview;
  onOpen: (subjectId: string) => void;
  t: Translate;
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
            <span>{t('subjects.notes')}</span>
            <span className="font-mono">
              {subject.knowledge_count}/{subject.topic_count}
            </span>
          </div>
          <Progress value={coverage} />
        </div>

        <div className="flex items-center justify-between">
          <div className="flex gap-4 text-xs text-muted-foreground">
            <span>{t('subjects.quizzes', { count: subject.quiz_count })}</span>
            <span>{t('subjects.accuracy', { percent: subject.accuracy_percent })}</span>
          </div>
          <Button size="sm" onClick={() => onOpen(subject.id)}>
            {t('subjects.open')}
            <ArrowRight className="size-4" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
