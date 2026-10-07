import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useInstallUpdate, useUpdateCheck } from "@/providers/queries";

function errorText(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return String(err);
}

/** Avviso di aggiornamento disponibile (PRD §36): controllo automatico, installazione solo su richiesta. */
export function UpdateBanner() {
  const { data: update } = useUpdateCheck();
  const install = useInstallUpdate();
  const [dismissed, setDismissed] = useState<string | null>(null);

  if (!update || dismissed === update.version) return null;

  return (
    <div role="status" className="flex items-center gap-3 border-b bg-accent px-4 py-2 text-sm">
      <span className="flex-1">
        È disponibile WinDozCal {update.version} (installata {update.current_version}).
        {install.isError && <span className="ml-2 text-destructive">Aggiornamento non riuscito: {errorText(install.error)}</span>}
      </span>
      <Button size="sm" disabled={install.isPending} onClick={() => install.mutate()}>
        {install.isPending ? "Download in corso…" : "Installa e riavvia"}
      </Button>
      <Button size="sm" variant="ghost" onClick={() => setDismissed(update.version)}>
        Più tardi
      </Button>
    </div>
  );
}
