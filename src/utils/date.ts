/** Primo giorno della settimana: 0 = domenica, 1 = lunedì (default PRD, locale IT). */
export type WeekStart = 0 | 1;

export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

/** Somma giorni di calendario (non millisecondi): resta corretto nei giorni di cambio ora legale. */
export function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n, d.getHours(), d.getMinutes(), d.getSeconds());
}

export function addMonths(d: Date, n: number): Date {
  const target = new Date(d.getFullYear(), d.getMonth() + n, 1);
  const lastDay = new Date(target.getFullYear(), target.getMonth() + 1, 0).getDate();
  return new Date(target.getFullYear(), target.getMonth(), Math.min(d.getDate(), lastDay));
}

export function startOfWeek(d: Date, weekStartsOn: WeekStart = 1): Date {
  const day = startOfDay(d);
  const diff = (day.getDay() - weekStartsOn + 7) % 7;
  return addDays(day, -diff);
}

/** I 7 giorni della settimana che contiene `d`, a partire da `weekStartsOn`. */
export function getWeekDays(d: Date, weekStartsOn: WeekStart = 1): Date[] {
  const first = startOfWeek(d, weekStartsOn);
  return Array.from({ length: 7 }, (_, i) => addDays(first, i));
}

/** Griglia mensile di 6 settimane (42 giorni) che include i giorni di raccordo. */
export function getMonthGrid(d: Date, weekStartsOn: WeekStart = 1): Date[] {
  const first = startOfWeek(new Date(d.getFullYear(), d.getMonth(), 1), weekStartsOn);
  return Array.from({ length: 42 }, (_, i) => addDays(first, i));
}

export function isSameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export function formatMonthYear(d: Date): string {
  return capitalize(new Intl.DateTimeFormat("it-IT", { month: "long", year: "numeric" }).format(d));
}

export function formatWeekday(d: Date, style: "short" | "long" = "short"): string {
  return capitalize(new Intl.DateTimeFormat("it-IT", { weekday: style }).format(d));
}

export function formatTime(d: Date): string {
  return new Intl.DateTimeFormat("it-IT", { hour: "2-digit", minute: "2-digit" }).format(d);
}

export function localTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone;
}

const pad = (n: number) => String(n).padStart(2, "0");

/** ISO 8601 con offset locale, es. `2026-10-06T09:00:00+02:00` (formato dei campi `start`/`end`). */
export function toIsoWithOffset(d: Date): string {
  const offset = -d.getTimezoneOffset();
  const sign = offset >= 0 ? "+" : "-";
  const abs = Math.abs(offset);
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
    `T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}` +
    `${sign}${pad(Math.floor(abs / 60))}:${pad(abs % 60)}`
  );
}
