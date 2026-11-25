import { useState } from "react";
import { X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n";
import type { NewAttendee } from "@/types";
import { validateAttendees } from "@/utils/validation";

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

interface Props {
  value: NewAttendee[];
  onChange: (value: NewAttendee[]) => void;
  /** Se presente la lista è in sola lettura e la nota spiega perché (calendari CalDAV, ADR 023). */
  readOnlyNote?: string;
}

/** Lista partecipanti: email obbligatoria, nome opzionale. Gli errori compaiono all'aggiunta. */
export function AttendeesField({ value, onChange, readOnlyNote }: Props) {
  const t = useT();
  const [email, setEmail] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);

  const add = () => {
    if (!email.trim() && !name.trim()) return;
    const candidate: NewAttendee = { email: email.trim(), name: name.trim() || null };
    const [problem] = validateAttendees([...value, candidate]);
    if (problem) return setError(problem);
    onChange([...value, candidate]);
    setEmail("");
    setName("");
    setError(null);
  };

  return (
    <div className="space-y-2">
      <span className="text-[11px] font-bold tracking-[0.1em] text-muted-foreground uppercase">{t("events.attendees.title")}</span>
      {value.length > 0 && (
        <ul className="space-y-1">
          {value.map((a) => (
            <li key={a.email} className="flex items-center gap-2 text-sm">
              <span className="truncate">
                {a.name ? `${a.name} ` : ""}
                <span className="text-muted-foreground">{a.name ? `<${a.email}>` : a.email}</span>
              </span>
              {!readOnlyNote && (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="ml-auto"
                  aria-label={t("events.attendees.remove", { email: a.email })}
                  onClick={() => onChange(value.filter((x) => x !== a))}
                >
                  <X />
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      {readOnlyNote ? (
        <p className="text-xs text-muted-foreground">{readOnlyNote}</p>
      ) : (
        <div
          className="flex gap-2"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              add();
            }
          }}
        >
          <input type="email" placeholder={t("events.attendees.email")} aria-label={t("events.attendees.emailLabel")} value={email} onChange={(e) => setEmail(e.target.value)} className={inputClass} />
          <input placeholder={t("events.attendees.name")} aria-label={t("events.attendees.nameLabel")} value={name} onChange={(e) => setName(e.target.value)} className={inputClass} />
          <Button type="button" variant="outline" onClick={add}>
            {t("events.attendees.add")}
          </Button>
        </div>
      )}
      {!readOnlyNote && error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
