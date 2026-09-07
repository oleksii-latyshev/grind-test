import { useEffect, useState } from "react";
import { ArrowLeft, Check, FileWarning, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { NoteReader } from "@/components/NoteReader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { api, errorMessage } from "@/lib/api";
import type { KnowledgeNote, Topic } from "@/lib/types";

interface Props {
  topic: Topic;
  onBack: () => void;
}

/** Standalone reading of one topic, opened from the topic list. */
export function ReaderScreen({ topic, onBack }: Props) {
  const [note, setNote] = useState<KnowledgeNote | null>(null);
  const [loading, setLoading] = useState(true);
  const [marked, setMarked] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    api
      .getKnowledge(topic.subject, topic.id)
      .then((result) => {
        if (!cancelled) setNote(result);
      })
      .catch((error) => {
        if (!cancelled) toast.error(errorMessage(error));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [topic]);

  async function markRead() {
    try {
      await api.markTopicRead(topic.id);
      setMarked(true);
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 p-8">
      <Button variant="ghost" size="sm" onClick={onBack} className="-ml-2">
        <ArrowLeft className="size-4" />
        Назад
      </Button>

      {loading ? (
        <div className="flex items-center gap-2 py-16 text-sm text-muted-foreground">
          <Loader2 className="size-4 animate-spin" />
          Завантаження конспекту…
        </div>
      ) : note ? (
        <>
          <NoteReader
            note={note}
            eyebrow={
              <Badge variant="outline" className="font-mono text-[0.7rem]">
                {topic.id}
              </Badge>
            }
          />
          <div className="mx-auto flex w-full max-w-3xl justify-end border-t border-border pt-6">
            <Button variant={marked ? "outline" : "default"} onClick={markRead} disabled={marked}>
              <Check className="size-4" />
              {marked ? "Позначено прочитаним" : "Позначити прочитаним"}
            </Button>
          </div>
        </>
      ) : (
        <div className="flex flex-col items-center gap-2 py-16 text-center text-sm text-muted-foreground">
          <FileWarning className="size-6" />
          <p>Конспект для цієї теми ще не згенеровано.</p>
        </div>
      )}
    </div>
  );
}
