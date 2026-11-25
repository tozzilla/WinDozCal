import { useState } from "react";
import { Button } from "@/components/ui/button";
import { errorText, useT } from "@/i18n";
import { useInstallUpdate, useUpdateCheck } from "@/providers/queries";

/** Avviso di aggiornamento disponibile (PRD §36): controllo automatico, installazione solo su richiesta. */
export function UpdateBanner() {
  const t = useT();
  const { data: update } = useUpdateCheck();
  const install = useInstallUpdate();
  const [dismissed, setDismissed] = useState<string | null>(null);

  if (!update || dismissed === update.version) return null;

  return (
    <div role="status" className="flex items-center gap-3 border-b bg-accent px-4 py-2 text-sm">
      <span className="flex-1">
        {t("shell.update.available", { version: update.version, current: update.current_version })}
        {install.isError && <span className="ml-2 text-destructive">{t("shell.update.failed", { error: errorText(install.error) })}</span>}
      </span>
      <Button size="sm" disabled={install.isPending} onClick={() => install.mutate()}>
        {install.isPending ? t("shell.update.downloading") : t("shell.update.install")}
      </Button>
      <Button size="sm" variant="ghost" onClick={() => setDismissed(update.version)}>
        {t("shell.update.later")}
      </Button>
    </div>
  );
}
