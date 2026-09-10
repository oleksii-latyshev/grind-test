import { FolderLock, FolderOpen, Loader2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { useT } from '@/i18n';
import type { VaultInfo } from '@/lib/types';

/**
 * Shown when the app cannot read the vault — most often because a macOS folder-access
 * prompt was declined. It spells out what picking a folder actually grants, so the choice
 * is informed rather than a blind retry of the same dialog.
 */
export function VaultCard({
  vault,
  busy,
  onChoose,
}: {
  vault: VaultInfo;
  busy: boolean;
  onChoose: () => void;
}) {
  const t = useT();
  return (
    <Card className="border-destructive/40">
      <CardContent className="space-y-4 py-6">
        <div className="flex items-start gap-3">
          <FolderLock className="mt-0.5 size-5 shrink-0 text-destructive" />
          <div className="space-y-1">
            <h2 className="text-sm font-semibold">{t('vault.denied.title')}</h2>
            <p className="text-sm leading-relaxed text-muted-foreground">{vault.error}</p>
          </div>
        </div>

        <div className="space-y-2 rounded-4xl border border-border bg-muted/40 p-4 text-sm leading-relaxed">
          <p className="font-medium">{t('vault.grants.title')}</p>
          <ul className="list-disc space-y-1 pl-5 text-muted-foreground">
            <li>{t('vault.grants.read')}</li>
            <li>{t('vault.grants.write')}</li>
            <li>{t('vault.grants.nothing')}</li>
          </ul>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button onClick={onChoose} disabled={busy}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : <FolderOpen className="size-4" />}
            {t('vault.choose')}
          </Button>
          <p className="font-mono text-xs text-muted-foreground">{vault.root}</p>
        </div>
      </CardContent>
    </Card>
  );
}
