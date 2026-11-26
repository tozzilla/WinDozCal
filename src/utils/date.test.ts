import { describe, expect, it } from "vitest";
import { getWeekDays, toIsoWithOffset } from "./date";

const ymd = (d: Date) => [d.getFullYear(), d.getMonth() + 1, d.getDate()];

describe("getWeekDays", () => {
  it("restituisce lunedì-domenica per una data a metà settimana", () => {
    const days = getWeekDays(new Date(2026, 9, 6)); // martedì 6 ottobre 2026
    expect(days).toHaveLength(7);
    expect(ymd(days[0])).toEqual([2026, 10, 5]);
    expect(ymd(days[6])).toEqual([2026, 10, 11]);
  });

  it("assegna la domenica alla settimana che finisce quel giorno", () => {
    const days = getWeekDays(new Date(2026, 9, 11));
    expect(ymd(days[0])).toEqual([2026, 10, 5]);
  });

  it("attraversa la fine dell'ora legale (25 ottobre 2026) senza saltare giorni", () => {
    const days = getWeekDays(new Date(2026, 9, 21));
    expect(days.map((d) => d.getDate())).toEqual([19, 20, 21, 22, 23, 24, 25]);
  });

  it("scavalca il mese", () => {
    const days = getWeekDays(new Date(2026, 9, 28));
    expect(days.map((d) => d.getDate())).toEqual([26, 27, 28, 29, 30, 31, 1]);
  });

  it("rispetta la domenica come primo giorno", () => {
    const days = getWeekDays(new Date(2026, 9, 6), 0);
    expect(ymd(days[0])).toEqual([2026, 10, 4]);
  });
});

describe("toIsoWithOffset", () => {
  it("produce ISO 8601 con offset che si rilegge allo stesso istante", () => {
    const d = new Date(2026, 9, 6, 9, 30);
    const iso = toIsoWithOffset(d);
    expect(iso).toMatch(/^2026-10-06T09:30:00[+-]\d{2}:\d{2}$/);
    expect(new Date(iso).getTime()).toBe(d.getTime());
  });
});

describe("eventi tutto il giorno con data senza ora (Google, CalDAV)", () => {
  it("restano sul loro giorno e non invadono il successivo", async () => {
    const { parseEventDate } = await import("./date");
    const { eventsOnDay } = await import("./events");
    expect(ymd(parseEventDate("2026-12-08"))).toEqual([2026, 12, 8]);
    expect(parseEventDate("2026-12-08").getHours()).toBe(0);
    const e = { id: "h", start: "2026-12-08", end: "2026-12-09", all_day: true, title: "Immacolata" } as unknown as import("@/types").Event;
    expect(eventsOnDay([e], new Date(2026, 11, 8))).toHaveLength(1);
    expect(eventsOnDay([e], new Date(2026, 11, 9))).toHaveLength(0);
    expect(eventsOnDay([e], new Date(2026, 11, 7))).toHaveLength(0);
  });
});
