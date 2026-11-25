import { useState } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n";
import type { NewReminder, ReminderType } from "@/types";
import { validateReminders } from "@/utils/validation";

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

interface Props {
  value: NewReminder[];
  onChange: (value: NewReminder[]) => void;
}

/** Lista promemoria: minuti prima dell'inizio e tipo (popup/email). Nessuno di default. */
export function RemindersField({ value, onChange }: Props) {
  const t = useT();
  const [minutes, setMinutes] = useState("10");
  const [type, setType] = useState<ReminderType>("popup");
  const [error, setError] = useState<string | null>(null);

  const add = () => {
    const candidate: NewReminder = { minutes_before: minutes.trim() === "" ? NaN : Number(minutes), type };
    const [problem] = validateReminders([candidate]);
    if (problem) return setError(problem);
    if (value.some((r) => r.minutes_before === candidate.minutes_before && r.type === type)) return setError(t("events.reminders.duplicate"));
    onChange([...value, candidate]);
    setError(null);
  };

  return (
    <div className="space-y-2">
      <span className="text-[11px] font-bold tracking-[0.1em] text-muted-foreground uppercase">{t("events.reminders.title")}</span>
      {value.length > 0 && (
        <ul className="space-y-1">
          {value.map((r) => (
            <li key={`${r.type}-${r.minutes_before}`} className="flex items-center gap-2 text-sm">
              {t("events.reminders.before", { count: r.minutes_before, type: r.type === "popup" ? t("events.reminders.typePopup") : t("events.reminders.typeEmail") })}
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="ml-auto"
                aria-label={t("events.reminders.remove", { count: r.minutes_before })}
                onClick={() => onChange(value.filter((x) => x !== r))}
              >
                <X />
              </Button>
            </li>
          ))}
        </ul>
      )}
      <div
        className="flex gap-2"
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            add();
          }
        }}
      >
        <input type="number" min={0} step={1} aria-label={t("events.reminders.minutesLabel")} value={minutes} onChange={(e) => setMinutes(e.target.value)} className={inputClass} />
        <select aria-label={t("events.reminders.typeLabel")} value={type} onChange={(e) => setType(e.target.value as ReminderType)} className={inputClass}>
          <option value="popup">{t("events.reminders.optionPopup")}</option>
          <option value="email">{t("events.reminders.optionEmail")}</option>
        </select>
        <Button type="button" variant="outline" onClick={add}>
          {t("events.reminders.add")}
        </Button>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
