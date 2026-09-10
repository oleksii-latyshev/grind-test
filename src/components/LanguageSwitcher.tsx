import { Languages } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { Locale } from '@/i18n';
import { LOCALES, useT } from '@/i18n';
import { cn } from '@/lib/utils';

/**
 * Interface language. Three options is few enough to cycle through rather than open a menu
 * for, and the current one is spelled out beside the icon so the next press is predictable.
 */
export function LanguageSwitcher({
  locale,
  onChange,
}: {
  locale: Locale;
  onChange: (locale: Locale) => void;
}) {
  const t = useT();
  const index = LOCALES.findIndex((entry) => entry.value === locale);
  const next = LOCALES[(index + 1) % LOCALES.length];

  return (
    <Button
      variant="ghost"
      size="sm"
      className="gap-1.5 px-2"
      onClick={() => onChange(next.value)}
      title={`${t('app.language')}: ${next.label}`}
    >
      <Languages className="size-4" />
      <span className={cn('text-xs uppercase', 'tracking-wide')}>{locale}</span>
    </Button>
  );
}
