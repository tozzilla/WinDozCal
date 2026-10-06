import { rruleParams, splitLines, WEEKDAYS } from "@/events/recurrence";
import type { Event } from "@/types";
import { toIsoWithOffset } from "@/utils/date";

// Espansione delle serie per il fallback in-memory (stesso comportamento del backend, in versione semplice):
// DAILY, WEEKLY con BYDAY, MONTHLY, YEARLY con INTERVAL/COUNT/UNTIL, più EXDATE.

const MAX_PER_SERIES = 500;
const MAX_ITERATIONS = 20_000;

const pad = (n: number) => String(n).padStart(2, "0");

/** Istante UTC come in una EXDATE: `YYYYMMDDTHHMMSSZ`. */
export const utcStamp = (d: Date) => d.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}/, "");
const dateStamp = (d: Date) => `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;

/** Valore EXDATE per un'occorrenza: `YYYYMMDD` per gli eventi all-day, istante UTC altrimenti. */
export function exdateValue(occurrenceStart: string, allDay: boolean): string {
  const d = new Date(occurrenceStart);
  return allDay ? dateStamp(d) : utcStamp(d);
}

function parseUntil(value: string | undefined): Date | null {
  const m = value?.match(/^(\d{4})(\d{2})(\d{2})(?:T(\d{2})(\d{2})(\d{2})Z?)?$/);
  if (!m) return null;
  const [, y, mo, d, h, mi, s] = m;
  return h ? new Date(Date.UTC(+y, +mo - 1, +d, +h, +mi, +s)) : new Date(+y, +mo - 1, +d, 23, 59, 59);
}

function exdates(rule: string): Set<string> {
  const out = new Set<string>();
  for (const line of splitLines(rule)) {
    if (!/^EXDATE/i.test(line)) continue;
    for (const v of line.slice(line.lastIndexOf(":") + 1).split(",")) out.add(v.trim());
  }
  return out;
}

function* candidates(start: Date, freq: string, interval: number, byDay: number[]): Generator<Date> {
  const [y, m, d, h, mi, s] = [start.getFullYear(), start.getMonth(), start.getDate(), start.getHours(), start.getMinutes(), start.getSeconds()];
  const at = (year: number, month: number, day: number) => new Date(year, month, day, h, mi, s);
  const daysIn = (year: number, month: number) => new Date(year, month + 1, 0).getDate();

  if (freq === "DAILY") {
    for (let k = 0; ; k++) yield at(y, m, d + k * interval);
  } else if (freq === "WEEKLY") {
    const mondayOffset = (start.getDay() + 6) % 7;
    const days = (byDay.length > 0 ? byDay : [mondayOffset]).slice().sort((a, b) => a - b);
    for (let w = 0; ; w++) {
      for (const offset of days) {
        const cand = at(y, m, d - mondayOffset + w * interval * 7 + offset);
        if (cand >= start) yield cand;
      }
    }
  } else if (freq === "MONTHLY") {
    for (let k = 0; ; k++) {
      const first = new Date(y, m + k * interval, 1);
      if (d <= daysIn(first.getFullYear(), first.getMonth())) yield at(first.getFullYear(), first.getMonth(), d);
    }
  } else if (freq === "YEARLY") {
    for (let k = 0; ; k++) {
      const year = y + k * interval;
      if (d <= daysIn(year, m)) yield at(year, m, d);
    }
  }
}

/** Occorrenze di `event` che si sovrappongono a [from, to). Un evento non ricorrente torna com'è. */
export function expandEvent(event: Event, from: Date, to: Date): Event[] {
  const start = new Date(event.start);
  const duration = new Date(event.end).getTime() - start.getTime();
  const overlaps = (s: Date) => s < to && new Date(s.getTime() + duration) > from;

  const params = rruleParams(event.recurrence_rule);
  const freq = params.get("FREQ")?.toUpperCase();
  if (!event.recurrence_rule || !freq || !["DAILY", "WEEKLY", "MONTHLY", "YEARLY"].includes(freq)) {
    return overlaps(start) ? [{ ...event, occurrence_start: null }] : [];
  }

  const interval = Math.max(1, parseInt(params.get("INTERVAL") ?? "1", 10) || 1);
  const count = parseInt(params.get("COUNT") ?? "", 10) || null;
  const until = parseUntil(params.get("UNTIL"));
  const byDay = (params.get("BYDAY") ?? "")
    .split(",")
    .map((d) => WEEKDAYS.indexOf(d as (typeof WEEKDAYS)[number]))
    .filter((i) => i >= 0);
  const excluded = exdates(event.recurrence_rule);

  const result: Event[] = [];
  let generated = 0;
  let iterations = 0;
  for (const cand of candidates(start, freq, interval, byDay)) {
    if (++iterations > MAX_ITERATIONS || cand >= to) break;
    if ((until && cand > until) || (count !== null && generated >= count)) break;
    generated++;
    if (!overlaps(cand)) continue;
    if (excluded.has(event.all_day ? dateStamp(cand) : utcStamp(cand))) continue;
    result.push({
      ...event,
      start: toIsoWithOffset(cand),
      end: toIsoWithOffset(new Date(cand.getTime() + duration)),
      occurrence_start: toIsoWithOffset(cand),
    });
    if (result.length >= MAX_PER_SERIES) break;
  }
  return result;
}
