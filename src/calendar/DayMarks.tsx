import { useT } from "@/i18n";
import { cn } from "@/lib/utils";
import type { DayAnnotation } from "@/utils/annotations";
import { formatTime } from "@/utils/date";
import type { MoonPhase } from "@/utils/moon";

/** Disco lunare come appare dall'emisfero nord: contorno = parte illuminata, pieno = ombra. */
function MoonDisc({ phase, className }: { phase: MoonPhase; className?: string }) {
  const rightHalf = "M8 1.5a6.5 6.5 0 0 1 0 13Z";
  return (
    <svg viewBox="0 0 16 16" className={cn("size-3.5 shrink-0", className)} aria-hidden>
      <circle cx="8" cy="8" r="6.5" fill={phase === "new" ? "currentColor" : "none"} stroke="currentColor" strokeWidth="1.2" />
      {/* Primo quarto: illuminata la metà destra, in ombra la sinistra; ultimo quarto il contrario. */}
      {phase === "firstQuarter" && <path d={rightHalf} fill="currentColor" transform="matrix(-1 0 0 1 16 0)" />}
      {phase === "lastQuarter" && <path d={rightHalf} fill="currentColor" />}
    </svg>
  );
}

/** Icona della fase lunare del giorno, con nome e ora locale accessibili. */
export function MoonMark({ annotation, className }: { annotation?: DayAnnotation; className?: string }) {
  const t = useT();
  const moon = annotation?.moon;
  if (!moon) return null;
  // Il giorno è già assegnato sull'istante esatto; l'ora si arrotonda al minuto come nelle tabelle USNO.
  const time = formatTime(new Date(Math.round(moon.instant.getTime() / 60_000) * 60_000));
  const label = t("calendar.moon.at", { phase: t(`calendar.moon.${moon.phase}`), time });
  return (
    <span role="img" aria-label={label} title={label} className={cn("inline-flex text-muted-foreground", className)}>
      <MoonDisc phase={moon.phase} />
    </span>
  );
}

/** Nomi delle festività del giorno, su una riga troncata. */
export function HolidayLabel({ annotation, className }: { annotation?: DayAnnotation; className?: string }) {
  const t = useT();
  if (!annotation?.holidays.length) return null;
  const text = annotation.holidays.map((id) => t(`calendar.holidays.${id}`)).join(" · ");
  return (
    <div title={text} className={cn("truncate text-[11px] leading-tight font-semibold text-holiday", className)}>
      {text}
    </div>
  );
}

export const isHoliday = (a?: DayAnnotation) => !!a?.holidays.length;
