import { Button } from "@/components/ui/button";
import { useReadingSize } from "@/lib/reading";

/** The `A` / `A` pair readers put in the corner of a page. */
export function ReadingSizeControl() {
  const { size, canDecrease, canIncrease, decrease, increase } = useReadingSize();

  return (
    <div className="flex items-center gap-0.5 rounded-4xl border border-border bg-background/60 p-0.5">
      <Button
        variant="ghost"
        size="icon"
        className="size-7"
        onClick={decrease}
        disabled={!canDecrease}
        title="Зменшити шрифт"
      >
        <span className="text-[0.7rem] font-semibold leading-none">A</span>
        <span className="sr-only">Зменшити шрифт</span>
      </Button>
      <span className="w-6 text-center text-xs tabular-nums text-muted-foreground">{size}</span>
      <Button
        variant="ghost"
        size="icon"
        className="size-7"
        onClick={increase}
        disabled={!canIncrease}
        title="Збільшити шрифт"
      >
        <span className="text-base font-semibold leading-none">A</span>
        <span className="sr-only">Збільшити шрифт</span>
      </Button>
    </div>
  );
}
