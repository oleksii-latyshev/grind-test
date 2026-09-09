import { useState } from "react";
import { FolderOpen, FolderPlus, GraduationCap, HardDrive, Loader2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { api, errorMessage } from "@/lib/api";
import type { VaultInfo } from "@/lib/types";

type Action = "choose" | "create" | "default";

/**
 * Shown once, on the very first launch, before the app has touched a single file.
 *
 * The app used to resolve a vault path on its own and read it immediately, which on macOS
 * meant an unexplained Desktop-access prompt for a folder the student never mentioned. Now
 * nothing is read until this screen says where the vault is — and the only system panel that
 * appears is a folder picker the student asked for, which is also what grants the access.
 */
export function SetupScreen({
  vault,
  onReady,
}: {
  vault: VaultInfo;
  onReady: (info: VaultInfo) => void;
}) {
  const [busy, setBusy] = useState<Action | null>(null);

  async function run(action: Action, call: () => Promise<VaultInfo>) {
    setBusy(action);
    try {
      const info = await call();
      // A cancelled picker returns the unchanged state; stay on this screen.
      if (info.configured) onReady(info);
    } catch (error) {
      toast.error(errorMessage(error));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-6 p-8 pt-16">
      <header className="space-y-3 text-center">
        <GraduationCap className="mx-auto size-10 text-primary" />
        <h1 className="text-2xl font-semibold tracking-tight">Вітаємо в grind-test</h1>
        <p className="text-sm leading-relaxed text-muted-foreground">
          Спершу оберіть, де зберігати ваші матеріали. Це єдина тека, до якої застосунок
          матиме доступ: там лежатимуть силабуси, згенеровані конспекти, тести й ваші
          результати. Більше нічого на диску він не читає.
        </p>
      </header>

      <div className="space-y-3">
        <Option
          icon={<FolderOpen className="size-5 text-primary" />}
          title="Обрати наявну теку"
          blurb="Якщо сховище вже є — вкажіть його. Підійде і тека, всередині якої лежить vault."
          action="Обрати теку"
          busy={busy === "choose"}
          disabled={busy !== null}
          onClick={() => run("choose", api.chooseVault)}
        />

        <Option
          icon={<FolderPlus className="size-5 text-primary" />}
          title="Створити нове сховище"
          blurb="Оберіть, де його створити. Всередині зʼявиться структура тек і приклад силабуса, щоб було видно формат."
          action="Створити"
          busy={busy === "create"}
          disabled={busy !== null}
          onClick={() => run("create", api.createVault)}
        />

        <Option
          icon={<HardDrive className="size-5 text-muted-foreground" />}
          title="Тримати всередині застосунку"
          blurb={vault.root}
          action="Використати"
          variant="outline"
          busy={busy === "default"}
          disabled={busy !== null}
          onClick={() => run("default", api.useDefaultVault)}
        />
      </div>

      <p className="text-center text-xs leading-relaxed text-muted-foreground">
        Вибір можна змінити будь-коли — на екрані предметів.
      </p>
    </div>
  );
}

function Option({
  icon,
  title,
  blurb,
  action,
  busy,
  disabled,
  variant,
  onClick,
}: {
  icon: React.ReactNode;
  title: string;
  blurb: string;
  action: string;
  busy: boolean;
  disabled: boolean;
  variant?: "outline";
  onClick: () => void;
}) {
  return (
    <Card>
      <CardContent className="flex flex-wrap items-center gap-4 py-5">
        <div className="shrink-0">{icon}</div>
        <div className="min-w-0 flex-1 space-y-1">
          <p className="text-sm font-medium">{title}</p>
          <p className="break-all text-xs leading-relaxed text-muted-foreground">{blurb}</p>
        </div>
        <Button variant={variant} onClick={onClick} disabled={disabled}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : null}
          {action}
        </Button>
      </CardContent>
    </Card>
  );
}
