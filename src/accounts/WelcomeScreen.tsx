import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useConnectMicrosoft, useCreateLocalAccount } from "@/providers/queries";
import { errorText } from "@/settings/AccountsSettings";
import { useUiStore } from "@/stores/uiStore";

/**
 * Schermata di primo avvio (PRD §33). Microsoft e modalità locale funzionano; l'autenticazione vive nel
 * backend (src-tauri/src/auth), il frontend chiede solo di avviarla. Google e CalDAV arrivano con i loro stage.
 */
export function WelcomeScreen() {
  const createLocal = useCreateLocalAccount();
  const connect = useConnectMicrosoft();
  const [error, setError] = useState<string | null>(null);
  const { setView, setCurrentDate } = useUiStore();

  const startLocal = async () => {
    await createLocal.mutateAsync("This computer");
    setView("week");
    setCurrentDate(new Date());
  };

  return (
    <div className="grid h-full grid-cols-[minmax(320px,44%)_1fr] bg-background">
      <section className="flex flex-col justify-between bg-rail p-10 text-rail-foreground">
        <span className="text-[17px] font-bold tracking-[0.02em]">WinDozCal</span>
        <div>
          <h1 className="max-w-md text-[44px] leading-[1.05] font-bold tracking-[-0.02em]">All your calendars. One simple desktop app.</h1>
          <p className="mt-4 max-w-sm text-[15px] text-rail-foreground/70">
            Every account becomes a channel. Open the app and see what is on now and what comes next.
          </p>
        </div>
        <p className="max-w-sm text-[12px] text-rail-foreground/60">
          Your calendar data is stored locally and synchronized directly with your provider. Without an account, it never leaves this computer.
        </p>
      </section>
      <section className="flex flex-col justify-center gap-3 p-10">
        <h2 className="mb-2 text-[13px] font-bold tracking-[0.12em] text-muted-foreground uppercase">Add your first channel</h2>
        <div className="flex w-80 flex-col gap-2">
          <Button size="lg" variant="outline" disabled>
            Continue with Google
          </Button>
          <Button
            size="lg"
            variant="outline"
            disabled={connect.isPending}
            onClick={() => {
              setError(null);
              connect.mutateAsync().catch((err) => setError(errorText(err)));
            }}
          >
            {connect.isPending ? "Complete sign-in in your browser…" : "Continue with Microsoft"}
          </Button>
          <Button size="lg" variant="outline" disabled>
            Add CalDAV account
          </Button>
          <Button size="lg" disabled={createLocal.isPending} onClick={() => void startLocal()}>
            Use without an account
          </Button>
        </div>
        {error && (
          <p role="alert" className="max-w-sm text-sm text-destructive">
            {error}
          </p>
        )}
      </section>
    </div>
  );
}
