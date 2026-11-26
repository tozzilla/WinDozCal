import { holidaysFor, isHolidayCountry, type HolidayId } from "./holidays";
import { moonPhases, type MoonPhaseEvent } from "./moon";
import { dayKey } from "./date";

/** Annotazioni di un giorno (ADR 024): dati derivati, non eventi. */
export interface DayAnnotation {
  holidays: HolidayId[];
  moon?: MoonPhaseEvent;
}

export type Annotations = ReadonlyMap<string, DayAnnotation>;

export const NO_ANNOTATIONS: Annotations = new Map();

export interface AnnotationOptions {
  holidayCountry: string | null;
  moonPhases: boolean;
  /** Giorno civile di un istante; default il fuso locale. I test passano un fuso esplicito. */
  dayOf?: (instant: Date) => string;
}

/**
 * Annotazioni dei giorni in `[start, end)`, indicizzate per `dayKey`. Le festività coprono ogni anno
 * toccato dal range; le fasi lunari si assegnano al giorno nel fuso locale.
 */
export function buildAnnotations(start: Date, end: Date, opts: AnnotationOptions): Annotations {
  const map = new Map<string, DayAnnotation>();
  const entry = (key: string) => {
    let a = map.get(key);
    if (!a) map.set(key, (a = { holidays: [] }));
    return a;
  };
  const country = opts.holidayCountry;
  if (isHolidayCountry(country)) {
    const from = dayKey(start);
    const to = dayKey(end);
    for (let y = start.getFullYear(); y <= end.getFullYear(); y++) {
      for (const h of holidaysFor(country, y)) {
        if (h.date >= from && h.date < to) entry(h.date).holidays.push(h.id);
      }
    }
  }
  if (opts.moonPhases) {
    for (const m of moonPhases(start, end)) entry((opts.dayOf ?? dayKey)(m.instant)).moon = m;
  }
  return map;
}
