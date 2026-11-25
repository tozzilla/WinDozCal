import { useId, useState, type FormEvent } from "react";
import { Button } from "@/components/ui/button";
import { errorText, useT } from "@/i18n";
import { useConnectCaldav, useUpdateCaldavPassword } from "@/providers/queries";

const inputClass =
  "w-full rounded-md border bg-background px-2.5 py-1.5 text-sm outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50";

/** Il server ha rifiutato il certificato: suggerisce l'importazione nell'archivio di Windows (ADR 023). */
const looksLikeCertificateError = (text: string) => /certificat|\btls\b|\bssl\b/i.test(text);

function FormError({ error }: { error: unknown }) {
  const t = useT();
  if (!error) return null;
  // Nel modulo un 401 significa credenziali sbagliate, non un accesso da rinnovare.
  const badCredentials = typeof error === "object" && (error as { code?: unknown }).code === "auth_required";
  const text = badCredentials ? t("accounts.caldav.badCredentials") : errorText(error);
  return (
    <div role="alert" className="space-y-1 text-sm text-destructive">
      <p>{text}</p>
      {looksLikeCertificateError(text) && <p className="text-xs">{t("accounts.caldav.certificateHint")}</p>}
    </div>
  );
}

/**
 * Modulo di collegamento CalDAV (ADR 023): URL del server, nome utente e password. La password vive solo nello
 * stato del modulo e si azzera a ogni invio e all'annullamento; il backend la salva in Credential Manager.
 */
export function CaldavForm({ onDone, onCancel }: { onDone?: () => void; onCancel: () => void }) {
  const t = useT();
  const id = useId();
  const connect = useConnectCaldav();
  const [serverUrl, setServerUrl] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const insecure = /^\s*http:\/\//i.test(serverUrl);
  const ready = serverUrl.trim() !== "" && username.trim() !== "" && password !== "";

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!ready || connect.isPending) return;
    const values = { serverUrl: serverUrl.trim(), username: username.trim(), password };
    setPassword("");
    connect.mutate(values, { onSuccess: () => onDone?.() });
  };

  const cancel = () => {
    setPassword("");
    onCancel();
  };

  return (
    <form onSubmit={submit} className="w-full max-w-sm space-y-3" aria-label={t("accounts.caldav.title")}>
      <h3 className="text-sm font-bold">{t("accounts.caldav.title")}</h3>
      <div className="space-y-1">
        <label htmlFor={`${id}-url`} className="text-xs font-medium">
          {t("accounts.caldav.serverUrl")}
        </label>
        <input
          id={`${id}-url`}
          type="url"
          required
          autoFocus
          autoComplete="off"
          spellCheck={false}
          placeholder="https://cloud.example.com/remote.php/dav"
          value={serverUrl}
          onChange={(e) => setServerUrl(e.target.value)}
          className={inputClass}
        />
        <p className="text-xs text-muted-foreground">{t("accounts.caldav.hint")}</p>
        {insecure && (
          <p role="alert" className="text-xs text-destructive">
            {t("accounts.caldav.insecureWarning")}
          </p>
        )}
      </div>
      <div className="space-y-1">
        <label htmlFor={`${id}-user`} className="text-xs font-medium">
          {t("accounts.caldav.username")}
        </label>
        <input id={`${id}-user`} type="text" required autoComplete="off" spellCheck={false} value={username} onChange={(e) => setUsername(e.target.value)} className={inputClass} />
      </div>
      <div className="space-y-1">
        <label htmlFor={`${id}-pwd`} className="text-xs font-medium">
          {t("accounts.caldav.password")}
        </label>
        <input id={`${id}-pwd`} type="password" required autoComplete="off" value={password} onChange={(e) => setPassword(e.target.value)} className={inputClass} />
        <p className="text-xs text-muted-foreground">{t("accounts.caldav.passwordHint")}</p>
      </div>
      <FormError error={connect.error} />
      <div className="flex gap-2">
        <Button type="submit" disabled={!ready || connect.isPending}>
          {connect.isPending ? t("accounts.caldav.connecting") : t("accounts.caldav.connect")}
        </Button>
        <Button type="button" variant="ghost" onClick={cancel}>
          {t("accounts.cancel")}
        </Button>
      </div>
    </form>
  );
}

/** Nuova password per un account CalDAV in `auth_required`, al posto del flusso OAuth. */
export function CaldavPasswordPrompt({ accountId, onDone, onCancel }: { accountId: string; onDone: () => void; onCancel: () => void }) {
  const t = useT();
  const id = useId();
  const update = useUpdateCaldavPassword();
  const [password, setPassword] = useState("");

  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (!password || update.isPending) return;
    const value = password;
    setPassword("");
    update.mutate({ accountId, password: value }, { onSuccess: onDone });
  };

  return (
    <form onSubmit={submit} className="flex w-full flex-wrap items-end gap-2" aria-label={t("accounts.caldav.newPassword")}>
      <div className="min-w-48 flex-1 space-y-1">
        <label htmlFor={`${id}-pwd`} className="text-xs font-medium">
          {t("accounts.caldav.newPassword")}
        </label>
        <input id={`${id}-pwd`} type="password" required autoFocus autoComplete="off" value={password} onChange={(e) => setPassword(e.target.value)} className={inputClass} />
      </div>
      <Button type="submit" size="sm" disabled={!password || update.isPending}>
        {t("accounts.caldav.savePassword")}
      </Button>
      <Button
        type="button"
        size="sm"
        variant="ghost"
        onClick={() => {
          setPassword("");
          onCancel();
        }}
      >
        {t("accounts.cancel")}
      </Button>
      <div className="w-full">
        <FormError error={update.error} />
      </div>
    </form>
  );
}
