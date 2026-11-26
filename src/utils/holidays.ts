/**
 * Festività nazionali calcolate in locale (ADR 024). Solo quelle valide in tutto il paese: niente
 * festività regionali o patronali, niente giorni decretati una tantum.
 *
 * Italia, fonti: legge 27 maggio 1949 n. 260 e modifiche (legge 5 marzo 1977 n. 54; DPR 28 dicembre
 * 1985 n. 792 per l'Epifania; legge 20 novembre 2000 n. 336 per il 2 giugno; legge 151/2025 per il
 * 4 ottobre, San Francesco d'Assisi, dal 2026). Elenco accurato dal 2001; per gli anni precedenti
 * mancano le regole abolite o spostate prima di allora.
 */

/** Paesi supportati. Il backend valida solo la forma del codice: un codice non in elenco vale "nessuno". */
export const HOLIDAY_COUNTRIES = ["IT"] as const;
export type HolidayCountry = (typeof HOLIDAY_COUNTRIES)[number];

export type HolidayId =
  | "newYear"
  | "epiphany"
  | "easter"
  | "easterMonday"
  | "liberation"
  | "labour"
  | "republic"
  | "assumption"
  | "saintFrancis"
  | "allSaints"
  | "immaculate"
  | "christmas"
  | "saintStephen";

export interface Holiday {
  /** Data civile `yyyy-mm-dd`. */
  date: string;
  id: HolidayId;
}

export function isHolidayCountry(code: string | null | undefined): code is HolidayCountry {
  return !!code && (HOLIDAY_COUNTRIES as readonly string[]).includes(code);
}

const pad = (n: number) => String(n).padStart(2, "0");
const ymd = (y: number, m: number, d: number) => `${y}-${pad(m)}-${pad(d)}`;

/** Domenica di Pasqua gregoriana (algoritmo anonimo Meeus/Jones/Butcher): `[mese 1-12, giorno]`. */
export function easterSunday(year: number): [number, number] {
  const a = year % 19;
  const b = Math.floor(year / 100);
  const c = year % 100;
  const d = Math.floor(b / 4);
  const e = b % 4;
  const f = Math.floor((b + 8) / 25);
  const g = Math.floor((b - f + 1) / 3);
  const h = (19 * a + b - d - g + 15) % 30;
  const i = Math.floor(c / 4);
  const k = c % 4;
  const l = (32 + 2 * e + 2 * i - h - k) % 7;
  const m = Math.floor((a + 11 * h + 22 * l) / 451);
  const month = Math.floor((h + l - 7 * m + 114) / 31);
  const day = ((h + l - 7 * m + 114) % 31) + 1;
  return [month, day];
}

interface FixedRule {
  id: HolidayId;
  month: number;
  day: number;
  /** Primo anno in cui la regola vale. */
  from?: number;
}

const ITALY_FIXED: FixedRule[] = [
  { id: "newYear", month: 1, day: 1 },
  { id: "epiphany", month: 1, day: 6, from: 1986 },
  { id: "liberation", month: 4, day: 25 },
  { id: "labour", month: 5, day: 1 },
  { id: "republic", month: 6, day: 2, from: 2001 },
  { id: "assumption", month: 8, day: 15 },
  { id: "saintFrancis", month: 10, day: 4, from: 2026 },
  { id: "allSaints", month: 11, day: 1 },
  { id: "immaculate", month: 12, day: 8 },
  { id: "christmas", month: 12, day: 25 },
  { id: "saintStephen", month: 12, day: 26 },
];

function italy(year: number): Holiday[] {
  const [em, ed] = easterSunday(year);
  // Lunedì dell'Angelo: aritmetica di calendario in UTC (nessun cambio d'ora).
  const monday = new Date(Date.UTC(year, em - 1, ed + 1));
  const list: Holiday[] = ITALY_FIXED.filter((r) => r.from === undefined || year >= r.from).map((r) => ({ id: r.id, date: ymd(year, r.month, r.day) }));
  list.push({ id: "easter", date: ymd(year, em, ed) });
  list.push({ id: "easterMonday", date: ymd(monday.getUTCFullYear(), monday.getUTCMonth() + 1, monday.getUTCDate()) });
  return list.sort((a, b) => a.date.localeCompare(b.date));
}

/** Festività nazionali di `country` nell'anno `year`, ordinate per data. */
export function holidaysFor(country: HolidayCountry, year: number): Holiday[] {
  switch (country) {
    case "IT":
      return italy(year);
  }
}
