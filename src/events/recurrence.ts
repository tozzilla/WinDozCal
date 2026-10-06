// Ricorrenze (docs/CONTRACT.md, "Ricorrenze"): `recurrence_rule` contiene righe RFC 5545 separate da un
// ritorno a capo, una `RRULE:...` ed eventuali `EXDATE:...`. Una regola senza prefisso (`FREQ=...`) vale come `RRULE:`.

export type RecurrenceFreq = "none" | "daily" | "weekly" | "monthly" | "yearly" | "custom";
export type Weekday = "MO" | "TU" | "WE" | "TH" | "FR" | "SA" | "SU";

export const WEEKDAYS: Weekday[] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];

export interface RecurrenceSpec {
  freq: RecurrenceFreq;
  /** Solo per `weekly`. */
  byDay: Weekday[];
}

const SIMPLE: RecurrenceFreq[] = ["daily", "weekly", "monthly", "yearly"];

export const splitLines = (rule: string | null) =>
  (rule ?? "")
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);

const isRRule = (line: string) => /^(RRULE:|FREQ=)/i.test(line);

export function rruleParams(rule: string | null): Map<string, string> {
  const line = splitLines(rule).find(isRRule);
  const params = new Map<string, string>();
  if (!line) return params;
  for (const part of line.replace(/^RRULE:/i, "").split(";")) {
    const [k, v] = part.split("=");
    if (k && v) params.set(k.toUpperCase(), v);
  }
  return params;
}

/** Legge la frequenza e i giorni; frequenze non gestite dall'editor (es. HOURLY) diventano `custom`. */
export function parseRecurrence(rule: string | null): RecurrenceSpec {
  const params = rruleParams(rule);
  const freqParam = params.get("FREQ")?.toLowerCase();
  if (!freqParam) return { freq: "none", byDay: [] };
  const freq = SIMPLE.find((f) => f === freqParam);
  if (!freq) return { freq: "custom", byDay: [] };
  const byDay = (params.get("BYDAY") ?? "")
    .split(",")
    .filter((d): d is Weekday => (WEEKDAYS as string[]).includes(d));
  return { freq, byDay: freq === "weekly" ? byDay : [] };
}

/**
 * Costruisce `recurrence_rule` dalla scelta dell'editor, conservando le righe che l'editor non gestisce
 * (`EXDATE`, ...) e i parametri della RRULE esistente (INTERVAL, COUNT, UNTIL, ...) se la frequenza non cambia.
 * `none` azzera la ricorrenza; `custom` lascia la regola com'è.
 */
export function buildRecurrenceRule(spec: RecurrenceSpec, existingRule: string | null): string | null {
  if (spec.freq === "none") return null;
  if (spec.freq === "custom") return existingRule;

  const existing = rruleParams(existingRule);
  const sameFreq = existing.get("FREQ")?.toLowerCase() === spec.freq;
  const params = new Map<string, string>([["FREQ", spec.freq.toUpperCase()]]);
  if (sameFreq) for (const [k, v] of existing) if (k !== "FREQ") params.set(k, v);
  if (spec.freq === "weekly") {
    if (spec.byDay.length > 0) params.set("BYDAY", WEEKDAYS.filter((d) => spec.byDay.includes(d)).join(","));
    else params.delete("BYDAY");
  } else if (!sameFreq) {
    params.delete("BYDAY");
  }

  const rrule = "RRULE:" + [...params].map(([k, v]) => `${k}=${v}`).join(";");
  const others = splitLines(existingRule).filter((l) => !isRRule(l));
  return [rrule, ...others].join("\n");
}

/** Giorno della settimana (`Weekday`) di una data locale. */
export function weekdayOf(d: Date): Weekday {
  return WEEKDAYS[(d.getDay() + 6) % 7];
}
