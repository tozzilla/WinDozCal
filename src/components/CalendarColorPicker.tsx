import { useState } from "react";
import { Check } from "lucide-react";
import { CALENDAR_COLORS } from "@/calendar/appearance";
import { cn } from "@/lib/utils";

/** Palette dei colori dei calendari (PRD §34) con campo esadecimale per un colore libero. */
export function CalendarColorPicker({ value, onPick, className }: { value: string; onPick: (color: string) => void; className?: string }) {
  const [hex, setHex] = useState(value);
  const valid = /^#[0-9a-fA-F]{6}$/.test(hex);
  return (
    <div className={cn("space-y-2", className)}>
      <div className="grid w-fit grid-cols-[repeat(7,24px)] gap-1.5">
        {CALENDAR_COLORS.map((c) => (
          <button
            key={c.value}
            type="button"
            title={c.label}
            aria-label={c.label}
            aria-pressed={value.toLowerCase() === c.value.toLowerCase()}
            onClick={() => onPick(c.value)}
            className="grid size-6 place-items-center rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring"
            style={{ background: c.value }}
          >
            {value.toLowerCase() === c.value.toLowerCase() && <Check className="size-3.5 text-white" strokeWidth={3} />}
          </button>
        ))}
      </div>
      <form
        className="flex items-center gap-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          if (valid) onPick(hex.toUpperCase());
        }}
      >
        <span aria-hidden className="size-6 shrink-0 rounded-full border" style={{ background: valid ? hex : "transparent" }} />
        <input
          value={hex}
          onChange={(e) => setHex(e.target.value.trim())}
          aria-label="Colore esadecimale"
          className="h-7 w-24 rounded-md border bg-card px-2 font-sans text-[13px] text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <button type="submit" disabled={!valid} className="h-7 rounded-md bg-primary px-2.5 text-[12px] font-semibold text-primary-foreground disabled:opacity-40">
          Usa
        </button>
      </form>
    </div>
  );
}
