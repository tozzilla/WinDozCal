import { Ban, Check } from "lucide-react";
import { EVENT_COLORS, EVENT_ICONS, EVENT_PATTERNS } from "@/calendar/appearance";
import { cn } from "@/lib/utils";

interface Appearance {
  color: string | null;
  icon: string | null;
  pattern: string | null;
}

const legend = "text-[11px] font-bold tracking-[0.1em] text-muted-foreground uppercase";
const choice =
  "grid size-8 place-items-center rounded-md border outline-none hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring aria-pressed:border-primary aria-pressed:ring-1 aria-pressed:ring-primary";

/** Aspetto del singolo evento (ADR 017): colore, icona e pattern. "Nessuno" torna al colore del calendario. */
export function AppearanceField({ value, calendarColor, onChange }: { value: Appearance; calendarColor: string; onChange: (v: Appearance) => void }) {
  return (
    <div className="space-y-3">
      <fieldset className="space-y-1">
        <legend className={legend}>Colore</legend>
        <div className="flex flex-wrap gap-1.5">
          <button
            type="button"
            title="Colore del calendario"
            aria-label="Colore del calendario"
            aria-pressed={value.color === null}
            onClick={() => onChange({ ...value, color: null })}
            className={cn(choice, "rounded-full")}
            style={{ background: calendarColor }}
          >
            {value.color === null && <Check className="size-4 text-white" strokeWidth={3} />}
          </button>
          {EVENT_COLORS.map((c) => (
            <button
              key={c.value}
              type="button"
              title={c.label}
              aria-label={c.label}
              aria-pressed={value.color === c.value}
              onClick={() => onChange({ ...value, color: c.value })}
              className={cn(choice, "rounded-full")}
              style={{ background: c.value }}
            >
              {value.color === c.value && <Check className="size-4 text-white" strokeWidth={3} />}
            </button>
          ))}
        </div>
      </fieldset>

      <fieldset className="space-y-1">
        <legend className={legend}>Icona</legend>
        <div className="flex flex-wrap gap-1.5">
          <button type="button" title="Nessuna icona" aria-label="Nessuna icona" aria-pressed={value.icon === null} onClick={() => onChange({ ...value, icon: null })} className={choice}>
            <Ban className="size-4 text-muted-foreground" />
          </button>
          {Object.entries(EVENT_ICONS).map(([key, { Icon, label }]) => (
            <button key={key} type="button" title={label} aria-label={label} aria-pressed={value.icon === key} onClick={() => onChange({ ...value, icon: key })} className={choice}>
              <Icon className="size-4" />
            </button>
          ))}
        </div>
      </fieldset>

      <fieldset className="space-y-1">
        <legend className={legend}>Riempimento</legend>
        <div className="flex flex-wrap gap-1.5">
          {[{ key: null, label: "Pieno" }, ...Object.entries(EVENT_PATTERNS).map(([key, p]) => ({ key, label: p.label }))].map(({ key, label }) => {
            const p = key ? EVENT_PATTERNS[key] : undefined;
            return (
              <button
                key={label}
                type="button"
                aria-pressed={value.pattern === key}
                onClick={() => onChange({ ...value, pattern: key })}
                className={cn(choice, "h-8 w-auto gap-2 px-2.5 text-[12px] font-semibold")}
              >
                <span
                  aria-hidden
                  className="size-4 rounded-[3px] border"
                  style={{
                    backgroundColor: `color-mix(in srgb, ${value.color ?? calendarColor} 25%, var(--card))`,
                    backgroundImage: p?.image,
                    backgroundSize: p?.size,
                  }}
                />
                {label}
              </button>
            );
          })}
        </div>
      </fieldset>
    </div>
  );
}
