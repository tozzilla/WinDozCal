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
