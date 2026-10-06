import * as chrono from "chrono-node";

export interface QuickAddResult {
  title: string;
  start: Date;
  end: Date | null;
}

/**
 * Predisposizione per Quick Add (PRD §8): estrae data/ora da testo libero in italiano.
 * Non ancora collegato alla UI.
 */
export function parseQuickAdd(text: string, reference: Date = new Date()): QuickAddResult | null {
  const [match] = chrono.it.parse(text, reference, { forwardDate: true });
  if (!match) return null;
  const title = (text.slice(0, match.index) + text.slice(match.index + match.text.length)).replace(/\s+/g, " ").trim();
  return { title, start: match.start.date(), end: match.end?.date() ?? null };
}
