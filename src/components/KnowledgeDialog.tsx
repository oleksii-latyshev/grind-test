import { useEffect, useState } from "react";
import { BookOpen, FileWarning, Loader2 } from "lucide-react";

import { Markdown } from "@/components/Markdown";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { api, errorMessage } from "@/lib/api";
import type { KnowledgeNote, Topic } from "@/lib/types";

interface Props {
  topic: Topic | null;
  onClose: () => void;
}

export function KnowledgeDialog({ topic, onClose }: Props) {
  const [note, setNote] = useState<KnowledgeNote | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!topic) return;
    let cancelled = false;
    setLoading(true);
    setNote(null);
    setError(null);
    api
      .getKnowledge(topic.subject, topic.id)
      .then((result) => {
        if (!cancelled) setNote(result);
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [topic]);

  return (
    <Dialog open={topic !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="flex max-h-[85vh] w-full flex-col gap-4 sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle className="pr-8 text-left leading-snug">{topic?.title}</DialogTitle>
          <DialogDescription className="text-left">
            {topic?.section_title}
            {note ? (
              <Badge variant="secondary" className="ml-2 align-middle font-mono text-[0.7rem]">
                {note.model}
              </Badge>
            ) : null}
          </DialogDescription>
        </DialogHeader>

        {loading ? (
          <div className="flex items-center gap-2 py-10 text-sm text-muted-foreground">
            <Loader2 className="size-4 animate-spin" />
            Завантаження конспекту…
          </div>
        ) : error ? (
          <p className="py-10 text-sm text-destructive">{error}</p>
        ) : note ? (
          <ScrollArea className="-mx-2 max-h-[65vh] px-2">
            <Markdown>{note.body}</Markdown>
          </ScrollArea>
        ) : (
          <div className="flex flex-col items-center gap-2 py-12 text-center text-sm text-muted-foreground">
            <FileWarning className="size-6" />
            <p>Конспект для цієї теми ще не згенеровано.</p>
            <p className="flex items-center gap-1.5">
              <BookOpen className="size-3.5" />
              Позначте тему і натисніть «Згенерувати конспекти».
            </p>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
