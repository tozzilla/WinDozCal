import { addDays, startOfDay } from "@/utils/date";

export const SNAP_MINUTES = 15;
const MINUTE = 60_000;

/** Arrotonda all'istante più vicino sulla griglia di `step` minuti. */
export function snapToStep(d: Date, step = SNAP_MINUTES): Date {
  const stepMs = step * MINUTE;
  return new Date(Math.round(d.getTime() / stepMs) * stepMs);
}

/**
 * Sposta di `dayDelta` giorni di calendario e `deltaMinutes`; l'inizio si aggancia alla griglia e la durata resta.
 * Un evento più corto di un giorno non esce dal giorno di destinazione.
 */
export function moveEvent(start: Date, end: Date, dayDelta: number, deltaMinutes: number, step = SNAP_MINUTES) {
  const duration = end.getTime() - start.getTime();
  const shifted = addDays(start, dayDelta);
  let newStart = snapToStep(new Date(shifted.getTime() + deltaMinutes * MINUTE), step);
  if (duration < 24 * 60 * MINUTE) {
    const dayStart = startOfDay(shifted).getTime();
    const latest = addDays(startOfDay(shifted), 1).getTime() - duration;
    newStart = new Date(Math.min(Math.max(newStart.getTime(), dayStart), latest));
  }
  return { start: newStart, end: new Date(newStart.getTime() + duration) };
}

/** Ridimensiona dal bordo inferiore: la fine si aggancia alla griglia, non scende sotto `step` e non supera la mezzanotte. */
export function resizeEvent(start: Date, end: Date, deltaMinutes: number, step = SNAP_MINUTES) {
  const midnight = addDays(startOfDay(start), 1);
  let newEnd = snapToStep(new Date(end.getTime() + deltaMinutes * MINUTE), step);
  if (end <= midnight && newEnd > midnight) newEnd = midnight;
  const minEnd = new Date(start.getTime() + step * MINUTE);
  return { start, end: newEnd < minEnd ? minEnd : newEnd };
}

/** Durata di un evento creato con un clic singolo su una fascia vuota. */
export const CLICK_EVENT_MINUTES = 60;

/**
 * Intervallo creato trascinando su una fascia vuota del giorno `day`: dai minuti `fromMinutes` ai minuti
 * `toMinutes` (anche verso l'alto), agganciati a `step`, almeno `step` minuti e dentro la giornata.
 */
export function slotRange(day: Date, fromMinutes: number, toMinutes: number, step = SNAP_MINUTES) {
  const clamp = (m: number) => Math.min(Math.max(m, 0), 24 * 60);
  const a = clamp(Math.floor(Math.min(fromMinutes, toMinutes) / step) * step);
  let b = clamp(Math.ceil(Math.max(fromMinutes, toMinutes) / step) * step);
  if (b - a < step) b = Math.min(a + step, 24 * 60);
  const base = startOfDay(day);
  const at = (m: number) => new Date(base.getFullYear(), base.getMonth(), base.getDate(), 0, m);
  return { start: at(b - a < step ? b - step : a), end: at(b) };
}

/** Evento di un clic singolo: dall'ora intera cliccata, `CLICK_EVENT_MINUTES` minuti (al massimo fino a mezzanotte). */
export function clickRange(day: Date, minutes: number) {
  const startMinutes = Math.min(Math.floor(minutes / 60) * 60, 23 * 60);
  return slotRange(day, startMinutes, startMinutes + CLICK_EVENT_MINUTES, CLICK_EVENT_MINUTES);
}
