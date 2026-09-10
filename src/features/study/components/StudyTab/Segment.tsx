import { cn } from '@/lib/utils';

interface Props {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

export function Segment({ active, onClick, children }: Props) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        'rounded-4xl border px-4 py-1.5 text-sm transition-colors',
        active
          ? 'border-primary bg-primary text-primary-foreground'
          : 'border-input bg-input/30 hover:bg-muted',
      )}
    >
      {children}
    </button>
  );
}
