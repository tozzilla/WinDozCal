/** Settimana ISO 8601 (ADR 024): da lunedì; la settimana 1 contiene il primo giovedì dell'anno. */
export interface IsoWeek {
  weekYear: number;
  week: number;
}

/** Settimana ISO della data civile locale di `d` (l'ora e il fuso non contano). */
export function isoWeek(d: Date): IsoWeek {
  // Si lavora in UTC sulla sola data civile: nessun effetto del cambio d'ora.
  const date = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const weekday = date.getUTCDay() || 7;
  // Il giovedì della stessa settimana decide l'anno ISO.
  date.setUTCDate(date.getUTCDate() + 4 - weekday);
  const weekYear = date.getUTCFullYear();
  const jan1 = Date.UTC(weekYear, 0, 1);
  const week = Math.floor((date.getTime() - jan1) / 86_400_000 / 7) + 1;
  return { weekYear, week };
}
