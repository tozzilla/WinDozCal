import { cn } from "@/lib/utils";
import { useSettings, useUpdateSettings } from "@/providers/queries";
import type { Settings } from "@/types";

interface SwitchRowProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

function SwitchRow({ label, description, checked, disabled, onChange }: SwitchRowProps) {
  return (
    <div className={cn("flex items-start justify-between gap-6 py-3", disabled && "opacity-50")}>
      <div>
        <div className="text-sm font-medium">{label}</div>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative mt-0.5 h-5 w-9 shrink-0 rounded-full border transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
          checked ? "border-primary bg-primary" : "bg-muted",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 size-3.5 rounded-full bg-background shadow transition-all",
            checked ? "left-[1.1rem]" : "left-0.5",
          )}
        />
      </button>
    </div>
  );
}

/** Sezione General (PRD §34): avvio e tray. Ogni modifica si salva subito; un errore resta visibile. */
export function GeneralSettings() {
  const { data: settings, isPending, error: loadError } = useSettings();
  const update = useUpdateSettings();

  if (isPending) return <p className="text-sm text-muted-foreground">Caricamento…</p>;
  if (!settings) return <p className="text-sm text-destructive">Impossibile leggere le impostazioni: {message(loadError)}</p>;

  const save = (patch: Partial<Settings>) => update.mutate({ ...settings, ...patch });

  return (
    <div className="max-w-xl">
      <div className="divide-y">
        <SwitchRow
          label="Start on Windows login"
          checked={settings.start_on_login}
          disabled={update.isPending}
          onChange={(v) => save({ start_on_login: v })}
        />
        <SwitchRow
          label="Start minimized"
          description="Vale solo per l'avvio automatico di Windows: se avvii WinDozCal a mano, la finestra si apre sempre."
          checked={settings.start_minimized}
          disabled={update.isPending || !settings.start_on_login}
          onChange={(v) => save({ start_minimized: v })}
        />
        <SwitchRow
          label="Keep running in the tray when the window is closed"
          description="Chiudendo la finestra l'app resta attiva nel tray; per uscire usa Quit dal menu del tray."
          checked={settings.close_to_tray}
          disabled={update.isPending}
          onChange={(v) => save({ close_to_tray: v })}
        />
      </div>
      {update.isError && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          Impostazione non salvata: {message(update.error)}
        </p>
      )}
      <p className="mt-6 text-sm text-muted-foreground">Primo giorno della settimana, orario di lavoro e durata predefinita: non ancora implementati.</p>
    </div>
  );
}

function message(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return "errore sconosciuto";
}
