import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useAccounts, useConnectMicrosoft, useDisconnectAccount, useReconnectAccount, useSyncNow } from "@/providers/queries";
import type { Account } from "@/types";

const PROVIDER_LABEL: Record<Account["provider"], string> = {
  local: "Questo computer",
  google: "Google",
  microsoft: "Microsoft",
  caldav: "CalDAV",
};

const STATUS_LABEL: Record<string, string> = {
  idle: "Sincronizzato",
  syncing: "Sincronizzazione in corso",
  error: "Errore di sincronizzazione",
  auth_required: "Accesso da rinnovare",
};

const dateTime = new Intl.DateTimeFormat("it-IT", { dateStyle: "short", timeStyle: "short" });

export function errorText(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return String(err);
}

/** Sezione Accounts (PRD §34): account collegati con sync, reconnect e disconnect; aggiunta di un account Microsoft. */
export function AccountsSettings() {
  const { data: accounts = [] } = useAccounts();
  const connect = useConnectMicrosoft();
  const reconnect = useReconnectAccount();
  const disconnect = useDisconnectAccount();
  const syncNow = useSyncNow();
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = (action: Promise<unknown>) => {
    setError(null);
    action.catch((err) => setError(errorText(err)));
  };

  return (
    <div className="max-w-2xl space-y-4">
      <ul className="divide-y rounded-lg border">
        {accounts.map((a) => (
          <li key={a.id} className="flex flex-wrap items-center gap-3 p-3">
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm font-medium">
                {a.name} <span className="font-normal text-muted-foreground">· {PROVIDER_LABEL[a.provider]}</span>
              </div>
              <div className="truncate text-xs text-muted-foreground">
                {a.email && `${a.email} · `}
                {a.provider === "local"
                  ? "Dati solo su questo computer"
                  : `${STATUS_LABEL[a.sync_status] ?? a.sync_status}${a.last_sync ? ` · ultimo sync ${dateTime.format(new Date(a.last_sync))}` : ""}`}
              </div>
            </div>
            {a.provider !== "local" && confirming !== a.id && (
              <div className="flex gap-2">
                <Button size="sm" variant="outline" disabled={syncNow.isPending} onClick={() => run(syncNow.mutateAsync(a.id))}>
                  Sincronizza
                </Button>
                {a.provider === "microsoft" && (
                  <Button
                    size="sm"
                    variant={a.sync_status === "auth_required" ? "default" : "outline"}
                    disabled={reconnect.isPending}
                    onClick={() => run(reconnect.mutateAsync(a.id))}
                  >
                    Riconnetti
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => setConfirming(a.id)}>
                  Scollega
                </Button>
              </div>
            )}
            {confirming === a.id && (
              <div className="flex w-full flex-wrap items-center gap-2 text-sm">
                <span className="text-muted-foreground">Calendari ed eventi di questo account spariscono da WinDozCal; sul server non cambia nulla.</span>
                <Button
                  size="sm"
                  variant="destructive"
                  disabled={disconnect.isPending}
                  onClick={() => {
                    setConfirming(null);
                    run(disconnect.mutateAsync(a.id));
                  }}
                >
                  Scollega
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setConfirming(null)}>
                  Annulla
                </Button>
              </div>
            )}
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-3">
        <Button disabled={connect.isPending} onClick={() => run(connect.mutateAsync())}>
          Aggiungi account Microsoft
        </Button>
        {(connect.isPending || reconnect.isPending) && <span className="text-sm text-muted-foreground">Completa l&apos;accesso nel browser…</span>}
      </div>
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
