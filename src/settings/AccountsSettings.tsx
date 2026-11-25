import { useState } from "react";
import { CaldavForm, CaldavPasswordPrompt } from "@/accounts/CaldavForm";
import { Button } from "@/components/ui/button";
import { dateFormat, errorText, hasKey, useT, type TKey } from "@/i18n";
import {
  useAccounts,
  useConnectGoogle,
  useConnectMicrosoft,
  useDisconnectAccount,
  useFailedEvents,
  useProviderAvailability,
  useReconnectAccount,
  useRetryFailedEvents,
  useSyncNow,
} from "@/providers/queries";

/** Sezione Accounts (PRD §34): account collegati con sync, reconnect e disconnect; aggiunta di un account Google, Microsoft o CalDAV. */
export function AccountsSettings() {
  const t = useT();
  const { data: accounts = [] } = useAccounts();
  const connect = useConnectMicrosoft();
  const connectGoogle = useConnectGoogle();
  const { data: available } = useProviderAvailability();
  const reconnect = useReconnectAccount();
  const disconnect = useDisconnectAccount();
  const syncNow = useSyncNow();
  const { data: failed = [] } = useFailedEvents();
  const retryFailed = useRetryFailedEvents();
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [caldavOpen, setCaldavOpen] = useState(false);
  const [passwordFor, setPasswordFor] = useState<string | null>(null);

  const run = (action: Promise<unknown>) => {
    setError(null);
    action.catch((err) => setError(errorText(err)));
  };

  return (
    <div className="max-w-2xl space-y-4">
      {failed.length > 0 && (
        <div role="alert" className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-destructive/40 bg-destructive/5 p-3 text-sm">
          <span>
            {t("accounts.rejected", { count: failed.length })}
          </span>
          <Button variant="outline" disabled={retryFailed.isPending} onClick={() => run(retryFailed.mutateAsync(undefined))}>
            {t("accounts.retryAll")}
          </Button>
        </div>
      )}
      <ul className="divide-y rounded-lg border">
        {accounts.map((a) => (
          <li key={a.id} className="flex flex-wrap items-center gap-3 p-3">
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm font-medium">
                {a.name} <span className="font-normal text-muted-foreground">· {t(`accounts.providers.${a.provider}` as TKey)}</span>
              </div>
              <div className="truncate text-xs text-muted-foreground">
                {a.email && `${a.email} · `}
                {a.provider === "caldav" && a.server_url && `${a.server_url} · `}
                {a.provider === "local"
                  ? t("accounts.localOnly")
                  : `${hasKey(`accounts.status.${a.sync_status}`) ? t(`accounts.status.${a.sync_status}` as TKey) : a.sync_status}${a.last_sync ? ` · ${t("accounts.lastSync", { date: dateFormat({ dateStyle: "short", timeStyle: "short" }).format(new Date(a.last_sync)) })}` : ""}`}
              </div>
            </div>
            {a.provider !== "local" && confirming !== a.id && passwordFor !== a.id && (
              <div className="flex gap-2">
                <Button size="sm" variant="outline" disabled={syncNow.isPending} onClick={() => run(syncNow.mutateAsync(a.id))}>
                  {t("accounts.sync")}
                </Button>
                {(a.provider === "microsoft" || a.provider === "google") && (
                  <Button
                    size="sm"
                    variant={a.sync_status === "auth_required" ? "default" : "outline"}
                    disabled={reconnect.isPending}
                    onClick={() => run(reconnect.mutateAsync(a.id))}
                  >
                    {t("accounts.reconnect")}
                  </Button>
                )}
                {a.provider === "caldav" && (
                  <Button size="sm" variant={a.sync_status === "auth_required" ? "default" : "outline"} onClick={() => setPasswordFor(a.id)}>
                    {t(a.sync_status === "auth_required" ? "accounts.reconnect" : "accounts.caldav.changePassword")}
                  </Button>
                )}
                <Button size="sm" variant="ghost" onClick={() => setConfirming(a.id)}>
                  {t("accounts.disconnect")}
                </Button>
              </div>
            )}
            {passwordFor === a.id && <CaldavPasswordPrompt accountId={a.id} onDone={() => setPasswordFor(null)} onCancel={() => setPasswordFor(null)} />}
            {confirming === a.id && (
              <div className="flex w-full flex-wrap items-center gap-2 text-sm">
                <span className="text-muted-foreground">{t("accounts.disconnectNote")}</span>
                <Button
                  size="sm"
                  variant="destructive"
                  disabled={disconnect.isPending}
                  onClick={() => {
                    setConfirming(null);
                    run(disconnect.mutateAsync(a.id));
                  }}
                >
                  {t("accounts.disconnect")}
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setConfirming(null)}>
                  {t("accounts.cancel")}
                </Button>
              </div>
            )}
          </li>
        ))}
      </ul>
      <div className="flex items-center gap-3">
        {available?.google && (
          <Button disabled={connectGoogle.isPending} onClick={() => run(connectGoogle.mutateAsync())}>
            {t("accounts.addGoogle")}
          </Button>
        )}
        {available?.microsoft && (
          <Button variant="outline" disabled={connect.isPending} onClick={() => run(connect.mutateAsync())}>
            {t("accounts.addMicrosoft")}
          </Button>
        )}
        <Button variant="outline" disabled={caldavOpen} onClick={() => setCaldavOpen(true)}>
          {t("accounts.addCaldav")}
        </Button>
        {(connect.isPending || connectGoogle.isPending || reconnect.isPending) && <span className="text-sm text-muted-foreground">{t("accounts.completeInBrowser")}</span>}
      </div>
      {caldavOpen && <CaldavForm onDone={() => setCaldavOpen(false)} onCancel={() => setCaldavOpen(false)} />}
      {error && (
        <p role="alert" className="text-sm text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
