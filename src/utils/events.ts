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
