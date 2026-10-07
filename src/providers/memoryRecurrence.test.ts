import { describe, expect, it } from "vitest";
import type { Event } from "@/types";
import { toIsoWithOffset } from "@/utils/date";
import { expandEvent, exdateValue } from "./memoryRecurrence";

const at = (m: number, d: number, h = 9) => new Date(2026, m - 1, d, h);

const series = (rule: string, start = at(10, 5), extra: Partial<Event> = {}): Event => ({
  id: "s1", calendar_id: "c", remote_id: null, title: "Serie", description: null, location: null, conference_url: null,
  start: toIsoWithOffset(start), end: toIsoWithOffset(new Date(start.getTime() + 3_600_000)), timezone: "Europe/Rome",
  all_day: false, recurrence_rule: rule, status: "busy", etag: null, updated_at: null, sync_status: "synced",
  local_updated_at: null, remote_updated_at: null, occurrence_start: null, series_id: null, original_start: null, ...extra,
});

const days = (events: Event[]) => events.map((e) => new Date(e.start).getDate());

describe("expandEvent", () => {
  it("espande una serie settimanale per BYDAY nel range e salta le EXDATE", () => {
    const excluded = exdateValue(toIsoWithOffset(at(10, 7)), false);
    const rule = `RRULE:FREQ=WEEKLY;BYDAY=MO,WE\nEXDATE:${excluded}`;
    const occ = expandEvent(series(rule), at(10, 5, 0), at(10, 19, 0));
    expect(days(occ)).toEqual([5, 12, 14]); // il mercoledì 7 è escluso
    expect(occ.every((e) => e.id === "s1" && e.occurrence_start === e.start)).toBe(true);
  });

  it("rispetta INTERVAL e COUNT per le serie giornaliere", () => {
    const occ = expandEvent(series("RRULE:FREQ=DAILY;INTERVAL=2;COUNT=3"), at(10, 1, 0), at(11, 30, 0));
    expect(days(occ)).toEqual([5, 7, 9]);
  });

  it("salta i mesi senza il giorno 31 nelle serie mensili", () => {
    const occ = expandEvent(series("RRULE:FREQ=MONTHLY", at(1, 31)), at(1, 1, 0), at(6, 1, 0));
    expect(occ.map((e) => new Date(e.start).getMonth() + 1)).toEqual([1, 3, 5]);
  });

  it("restituisce un evento non ricorrente con occurrence_start null, solo se nel range", () => {
    const single = series("", at(10, 5));
    single.recurrence_rule = null;
    expect(expandEvent(single, at(10, 5, 0), at(10, 6, 0))[0].occurrence_start).toBeNull();
    expect(expandEvent(single, at(10, 6, 0), at(10, 7, 0))).toEqual([]);
  });
});
