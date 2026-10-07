import type { Event } from "@/types";
import { addDays, startOfDay } from "./date";

export function eventInterval(e: Event): { start: Date; end: Date } {
  const start = new Date(e.start);
  const end = new Date(e.end);
  return { start, end: end > start ? end : new Date(start.getTime() + 30 * 60_000) };
}

/** Eventi che si sovrappongono al giorno di calendario `day` (intervallo semiaperto [start, end)). */
export function eventsOnDay(events: Event[], day: Date): Event[] {
  const dayStart = startOfDay(day);
  const dayEnd = addDays(dayStart, 1);
  return events
    .filter((e) => {
      const { start, end } = eventInterval(e);
      return start < dayEnd && end > dayStart;
    })
    .sort((a, b) => a.start.localeCompare(b.start) || a.title.localeCompare(b.title));
}

/** Chiave univoca lato UI: le occorrenze di una serie condividono l'`id`. */
export const eventKey = (e: Event) => `${e.id}|${e.occurrence_start ?? ""}`;

export const isRecurring = (e: Event) => !!e.recurrence_rule;

/** Occorrenza di una serie: espansa (`id` della serie + `occurrence_start`) o eccezione (`series_id` + `original_start`). */
export interface OccurrenceRef {
  seriesId: string;
  occurrenceStart: string;
  /** `true` se l'occorrenza è già un'eccezione (riga a sé, modificabile con `update_event`). */
  isException: boolean;
}

export function occurrenceRef(e: Event): OccurrenceRef | null {
  if (e.series_id && e.original_start) return { seriesId: e.series_id, occurrenceStart: e.original_start, isException: true };
  if (e.occurrence_start) return { seriesId: e.id, occurrenceStart: e.occurrence_start, isException: false };
  return null;
}
