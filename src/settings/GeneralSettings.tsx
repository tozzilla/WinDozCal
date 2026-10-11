import { cn } from "@/lib/utils";
import { useSettings, useUpdateSettings } from "@/providers/queries";
import { DEFAULT_DURATIONS, type Settings } from "@/types";

const selectClass = "rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50";

/** Orari a passi di 30 minuti, in minuti dalla mezzanotte (limiti in docs/CONTRACT.md). */
const HALF_HOURS = Array.from({ length: 49 }, (_, i) => i * 30);
const hhmm = (m: number) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;
const durationLabel = (m: number) => (m < 60 ? `${m} minuti` : m === 60 ? "1 ora" : `${String(m / 60).replace(".", ",")} ore`);

function FieldRow({ label, description, children }: { label: string; description?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-6 py-3">
      <div>
        <div className="text-sm font-medium">{label}</div>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-2 text-sm">{children}</div>
    </div>
  );
}

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

/** Sezione General (PRD §34): avvio, tray e preferenze del calendario. Ogni modifica si salva subito; un errore resta visibile. */
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
        <FieldRow label="First day of week">
          <select
            aria-label="First day of week"
            className={selectClass}
            disabled={update.isPending}
            value={settings.week_start}
            onChange={(e) => save({ week_start: Number(e.target.value) as Settings["week_start"] })}
          >
            <option value={1}>Lunedì</option>
            <option value={0}>Domenica</option>
          </select>
        </FieldRow>
        <FieldRow label="Working hours" description="La griglia si apre all'inizio dell'orario; le ore fuori orario sono attenuate.">
          <select
            aria-label="Inizio dell'orario di lavoro"
            className={selectClass}
            disabled={update.isPending}
            value={settings.work_start}
            onChange={(e) => save({ work_start: Number(e.target.value) })}
          >
            {HALF_HOURS.filter((m) => m < settings.work_end).map((m) => (
              <option key={m} value={m}>
                {hhmm(m)}
              </option>
            ))}
          </select>
          <span aria-hidden>–</span>
          <select
            aria-label="Fine dell'orario di lavoro"
            className={selectClass}
            disabled={update.isPending}
            value={settings.work_end}
            onChange={(e) => save({ work_end: Number(e.target.value) })}
          >
            {HALF_HOURS.filter((m) => m > settings.work_start).map((m) => (
              <option key={m} value={m}>
                {hhmm(m)}
              </option>
            ))}
          </select>
        </FieldRow>
        <FieldRow label="Default event duration" description="Vale per il clic su una fascia vuota, il nuovo evento e Quick Add senza ora di fine.">
          <select
            aria-label="Default event duration"
            className={selectClass}
            disabled={update.isPending}
            value={settings.default_duration}
            onChange={(e) => save({ default_duration: Number(e.target.value) })}
          >
            {DEFAULT_DURATIONS.map((m) => (
              <option key={m} value={m}>
                {durationLabel(m)}
              </option>
            ))}
          </select>
        </FieldRow>
      </div>
      {update.isError && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          Impostazione non salvata: {message(update.error)}
        </p>
      )}
    </div>
  );
}

function message(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return "errore sconosciuto";
}
