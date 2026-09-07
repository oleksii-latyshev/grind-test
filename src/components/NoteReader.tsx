import { Markdown } from "@/components/Markdown";
import { Badge } from "@/components/ui/badge";
import type { KnowledgeNote } from "@/lib/types";

/**
 * The reading surface for a knowledge note: a single comfortable column rather than a
 * modal, because these notes run to a thousand words.
 */
export function NoteReader({
  note,
  eyebrow,
}: {
  note: KnowledgeNote;
  eyebrow?: React.ReactNode;
}) {
  return (
    <article className="mx-auto w-full max-w-3xl space-y-6">
      <header className="space-y-3 border-b border-border pb-6">
        {eyebrow ? <div className="flex items-center gap-2">{eyebrow}</div> : null}
        <h1 className="text-2xl font-semibold leading-tight tracking-tight">{note.title}</h1>
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <span>{note.section_title}</span>
          <Badge variant="secondary" className="font-mono text-[0.7rem]">
            {note.model}
          </Badge>
        </div>
      </header>

      <Markdown className="pb-8">{note.body}</Markdown>
    </article>
  );
}
