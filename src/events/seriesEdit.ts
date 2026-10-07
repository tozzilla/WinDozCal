import type { Event, NewEvent } from "@/types";
import { addDays, startOfDay, toIsoWithOffset } from "@/utils/date";

/** Portata di una modifica su un'occorrenza di serie (PRD §10, ADR 013). */
export type SeriesScope = "this" | "following" | "all";

export const SCOPE_LABEL: Record<SeriesScope, string> = {
  this: "Solo questo evento",
  following: "Questo e i successivi",
  all: "Tutta la serie",
};

const minutesOfDay = (d: Date) => d.getHours() * 60 + d.getMinutes();

/** Campi scrivibili di un evento esistente, come payload di `create_event` / `update_occurrence` / `split_series`. */
export function toNewEvent(e: Event): NewEvent {
  return {
    calendar_id: e.calendar_id,
    title: e.title,
    description: e.description,
    location: e.location,
    conference_url: e.conference_url,
    start: e.start,
    end: e.end,
    timezone: e.timezone,
    all_day: e.all_day,
    recurrence_rule: e.recurrence_rule,
    status: e.status,
  };
}

/**
 * "Tutta la serie" a partire da un'occorrenza modificata: la serie riceve i campi descrittivi e la regola,
 * e lo stesso spostamento fatto sull'occorrenza (giorni di calendario + ora locale, quindi corretto anche
 * a cavallo del cambio d'ora) con la nuova durata. Da un'eccezione gli orari della serie non cambiano.
 */
export function applyToSeries(series: Event, occurrenceStart: string, edited: NewEvent, fromException: boolean): Event {
  const base: Event = {
    ...series,
    title: edited.title,
    description: edited.description,
    location: edited.location,
    conference_url: edited.conference_url,
    status: edited.status,
    recurrence_rule: edited.recurrence_rule,
  };
  if (fromException) return base;

  const occ = new Date(occurrenceStart);
  const newStart = new Date(edited.start);
  const duration = new Date(edited.end).getTime() - newStart.getTime();
  const dayDelta = Math.round((startOfDay(newStart).getTime() - startOfDay(occ).getTime()) / 86_400_000);
  const seriesStart = new Date(series.start);
  const shifted = addDays(seriesStart, dayDelta);
  const start = edited.all_day
    ? startOfDay(shifted)
    : new Date(shifted.getFullYear(), shifted.getMonth(), shifted.getDate(), 0, minutesOfDay(seriesStart) + minutesOfDay(newStart) - minutesOfDay(occ));
  const end = edited.all_day ? addDays(start, Math.max(1, Math.round(duration / 86_400_000))) : new Date(start.getTime() + duration);
  return { ...base, all_day: edited.all_day, start: toIsoWithOffset(start), end: toIsoWithOffset(end) };
}
