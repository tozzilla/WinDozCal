import { Button } from "@/components/ui/button";
import { useCreateLocalAccount } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";

/**
 * Schermata di primo avvio (PRD §33). I pulsanti non sono ancora collegati:
 * l'autenticazione vivrà nel backend (src-tauri/src/auth), non nel frontend.
 */
export function WelcomeScreen() {
  const createLocal = useCreateLocalAccount();
  const { setView, setCurrentDate } = useUiStore();

  const startLocal = async () => {
    await createLocal.mutateAsync("This computer");
    setView("week");
    setCurrentDate(new Date());
  };

  return (
    <div className="flex h-full flex-col items-center justify-center gap-8 p-8 text-center">
      <div className="space-y-2">
        <h1 className="text-3xl font-semibold">Welcome to WinDozCal</h1>
        <p className="text-muted-foreground">
          All your calendars.
          <br />
          One simple desktop app.
        </p>
      </div>
      <div className="flex w-64 flex-col gap-2">
        <Button size="lg" disabled>
          Continue with Google
        </Button>
        <Button size="lg" variant="outline" disabled>
          Continue with Microsoft
        </Button>
        <Button size="lg" variant="outline" disabled>
          Add CalDAV account
        </Button>
        <Button size="lg" variant="secondary" disabled={createLocal.isPending} onClick={() => void startLocal()}>
          Use without an account
        </Button>
      </div>
      <p className="max-w-xs text-xs text-muted-foreground">
        Your calendar data is stored locally and synchronized directly with your provider. Without an account, it never leaves this computer.
      </p>
    </div>
  );
}
