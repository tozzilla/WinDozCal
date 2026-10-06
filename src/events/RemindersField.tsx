import { useState } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { NewReminder, ReminderType } from "@/types";
import { validateReminders } from "@/utils/validation";

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

const TYPE_LABEL: Record<ReminderType, string> = { popup: "notifica", email: "email" };

interface Props {
  value: NewReminder[];
  onChange: (value: NewReminder[]) => void;
}

/** Lista promemoria: minuti prima dell'inizio e tipo (popup/email). Nessuno di default. */
export function RemindersField({ value, onChange }: Props) {
  const [minutes, setMinutes] = useState("10");
  const [type, setType] = useState<ReminderType>("popup");
  const [error, setError] = useState<string | null>(null);

  const add = () => {
    const candidate: NewReminder = { minutes_before: minutes.trim() === "" ? NaN : Number(minutes), type };
    const [problem] = validateReminders([candidate]);
    if (problem) return setError(problem);
    if (value.some((r) => r.minutes_before === candidate.minutes_before && r.type === type)) return setError("Promemoria già presente.");
    onChange([...value, candidate]);
    setError(null);
  };

  return (
    <div className="space-y-2">
      <span className="text-xs text-muted-foreground">Promemoria</span>
      {value.length > 0 && (
        <ul className="space-y-1">
          {value.map((r) => (
            <li key={`${r.type}-${r.minutes_before}`} className="flex items-center gap-2 text-sm">
              {r.minutes_before} minuti prima ({TYPE_LABEL[r.type]})
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="ml-auto"
                aria-label={`Rimuovi promemoria ${r.minutes_before} minuti prima`}
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
        <input type="number" min={0} step={1} aria-label="Minuti prima" value={minutes} onChange={(e) => setMinutes(e.target.value)} className={inputClass} />
        <select aria-label="Tipo di promemoria" value={type} onChange={(e) => setType(e.target.value as ReminderType)} className={inputClass}>
          <option value="popup">Notifica</option>
          <option value="email">Email</option>
        </select>
        <Button type="button" variant="outline" onClick={add}>
          Aggiungi
        </Button>
      </div>
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
