import { useState } from "react";
import { Button } from "@/components/ui/button";
import { CaldavForm } from "./CaldavForm";
import { errorText, useT } from "@/i18n";
import { useConnectGoogle, useConnectMicrosoft, useCreateLocalAccount, useProviderAvailability } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";

/**
 * Schermata di primo avvio (PRD §33). Google, Microsoft e modalità locale funzionano; l'autenticazione vive nel
 * backend (src-tauri/src/auth), il frontend chiede solo di avviarla. CalDAV è sempre disponibile: modulo con URL, utente e password.
 */
export function WelcomeScreen() {
  const t = useT();
  const createLocal = useCreateLocalAccount();
  const connect = useConnectMicrosoft();
  const connectGoogle = useConnectGoogle();
  const { data: available } = useProviderAvailability();
  const [error, setError] = useState<string | null>(null);
  const [caldavOpen, setCaldavOpen] = useState(false);
  const { setView, setCurrentDate } = useUiStore();

  const startLocal = async () => {
    await createLocal.mutateAsync(t("accounts.localName"));
    setView("week");
    setCurrentDate(new Date());
  };

  return (
    <div className="grid h-full grid-cols-[minmax(320px,44%)_1fr] bg-background">
      <section className="flex flex-col justify-between bg-rail p-10 text-rail-foreground">
        <span className="text-[17px] font-bold tracking-[0.02em]">WinDozCal</span>
        <div>
          <h1 className="max-w-md text-[44px] leading-[1.05] font-bold tracking-[-0.02em]">{t("accounts.welcome.headline")}</h1>
          <p className="mt-4 max-w-sm text-[15px] text-rail-foreground/70">
            {t("accounts.welcome.tagline")}
          </p>
        </div>
        <p className="max-w-sm text-[12px] text-rail-foreground/60">
          {t("accounts.welcome.privacy")}
        </p>
      </section>
      <section className="flex flex-col justify-center gap-3 p-10">
        <h2 className="mb-2 text-[13px] font-bold tracking-[0.12em] text-muted-foreground uppercase">{t("accounts.welcome.addFirst")}</h2>
        {caldavOpen ? (
          <CaldavForm onCancel={() => setCaldavOpen(false)} />
        ) : (
        <div className="flex w-80 flex-col gap-2">
          {available?.google && (
            <Button
              size="lg"
              variant="outline"
              disabled={connectGoogle.isPending}
              onClick={() => {
                setError(null);
                connectGoogle.mutateAsync().catch((err) => setError(errorText(err)));
              }}
            >
              {connectGoogle.isPending ? t("accounts.completeInBrowser") : t("accounts.welcome.continueGoogle")}
            </Button>
          )}
          {available?.microsoft && (
            <Button
              size="lg"
              variant="outline"
              disabled={connect.isPending}
              onClick={() => {
                setError(null);
                connect.mutateAsync().catch((err) => setError(errorText(err)));
              }}
            >
              {connect.isPending ? t("accounts.completeInBrowser") : t("accounts.welcome.continueMicrosoft")}
            </Button>
          )}
          <Button
            size="lg"
            variant="outline"
            onClick={() => {
              setError(null);
              setCaldavOpen(true);
            }}
          >
            {t("accounts.welcome.addCaldav")}
          </Button>
          <Button size="lg" disabled={createLocal.isPending} onClick={() => void startLocal()}>
            {t("accounts.welcome.useLocal")}
          </Button>
        </div>
        )}
        {error && (
          <p role="alert" className="max-w-sm text-sm text-destructive">
            {error}
          </p>
        )}
      </section>
    </div>
  );
}
