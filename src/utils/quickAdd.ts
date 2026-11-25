import * as chrono from "chrono-node";
import { getLanguage } from "@/i18n";

export interface QuickAddResult {
  title: string;
  start: Date;
  end: Date;
  allDay: boolean;
}


const SECONDARY = { es: chrono.es, fr: chrono.fr, de: chrono.de } as const;

/** Il parser della lingua attiva (se secondaria) viene per primo, poi sempre italiano e inglese. */
function activeParsers() {
  const lang = getLanguage();
  const extra = lang === "es" || lang === "fr" || lang === "de" ? [SECONDARY[lang]] : [];
  return [...extra, chrono.it, chrono.en];
}

/**
 * Quick Add (PRD §8, ADR 012): estrae titolo, data e ora da testo libero in italiano, inglese o nella lingua secondaria attiva (es/fr/de), in locale.
 * Si prova ogni lingua e si tiene il riconoscimento che copre più testo ("venerdì 9:30" batte "9:30").
 * Senza orario l'evento è "tutto il giorno"; senza fine dura `defaultMinutes` (Settings › default_duration);
 * senza data riconosciuta restituisce `null`.
 */
export function parseQuickAdd(text: string, reference: Date = new Date(), defaultMinutes = 60): QuickAddResult | null {
  const matches = activeParsers()
    .map((p) => p.parse(text, reference, { forwardDate: true })[0])
    .filter((m): m is chrono.ParsedResult => !!m);
  if (matches.length === 0) return null;
  const match = matches.reduce((best, m) => (m.text.length > best.text.length ? m : best));

  const title = (text.slice(0, match.index) + text.slice(match.index + match.text.length))
    .replace(/\s+/g, " ")
    .replace(/\s+(alle|alle ore|at|on|il|la|di)$/i, "")
    .trim();
  // Un'ora implicita diversa da mezzogiorno viene da una parte del giorno ("sabato sera"): vale come orario.
  const timed = match.start.isCertain("hour") || match.start.get("hour") !== 12;

  if (!timed) {
    const start = match.start.date();
    const day = new Date(start.getFullYear(), start.getMonth(), start.getDate());
    return { title, start: day, end: new Date(day.getFullYear(), day.getMonth(), day.getDate() + 1), allDay: true };
  }
  const start = match.start.date();
  const end = match.end?.date();
  return {
    title,
    start,
    end: end && end > start ? end : new Date(start.getTime() + defaultMinutes * 60_000),
    allDay: false,
  };
}
