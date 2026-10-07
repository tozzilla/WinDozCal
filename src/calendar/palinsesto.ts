import type { Calendar, Event } from "@/types";
import { isSameDay } from "@/utils/date";
import { eventInterval, eventKey } from "@/utils/events";

/** Segni del palinsesto su un evento: in onda adesso, il prossimo di oggi, sovrapposto a un altro canale. */
export type BroadcastMark = "onair" | "next" | "conflict";

export const MARK_LABEL: Record<BroadcastMark, string> = {
  onair: "IN ONDA",
  next: "A SEGUIRE",
  conflict: "CONFLITTO",
};

/**
 * Calcola i segni per gli eventi con orario: IN ONDA per quelli in corso; CONFLITTO per un evento che inizia
 * mentre ne è in corso un altro di un account diverso (il doppio impegno tra canali); A SEGUIRE per il primo
 * evento di oggi, dopo adesso, che non ha già un segno. Un evento ha al massimo un segno, con questa precedenza.
 */
export function broadcastMarks(events: Event[], calendars: Calendar[], now: Date): Map<string, BroadcastMark> {
  const accountOf = new Map(calendars.map((c) => [c.id, c.account_id]));
  const timed = events
    .filter((e) => !e.all_day)
    .map((e) => ({ e, ...eventInterval(e) }))
    .sort((a, b) => a.start.getTime() - b.start.getTime());
  const marks = new Map<string, BroadcastMark>();

  for (const { e, start, end } of timed) {
    if (start <= now && end > now) marks.set(eventKey(e), "onair");
  }
  timed.forEach((later, i) => {
    if (marks.has(eventKey(later.e))) return;
    const clash = timed
      .slice(0, i)
      .some(
        (earlier) =>
          earlier.end > later.start &&
          earlier.start <= later.start &&
          accountOf.get(earlier.e.calendar_id) !== accountOf.get(later.e.calendar_id),
      );
    if (clash) marks.set(eventKey(later.e), "conflict");
  });
  const next = timed.find(({ e, start }) => start > now && isSameDay(start, now) && !marks.has(eventKey(e)));
  if (next) marks.set(eventKey(next.e), "next");
  return marks;
}
