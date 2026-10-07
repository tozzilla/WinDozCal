import { describe, expect, it } from "vitest";
import type { Event } from "@/types";
import { toIsoWithOffset } from "@/utils/date";
import { applyToSeries, toNewEvent } from "./seriesEdit";

const at = (m: number, d: number, h: number, min = 0) => toIsoWithOffset(new Date(2026, m - 1, d, h, min));

const series: Event = {
  id: "s", calendar_id: "c", remote_id: null, title: "Standup", description: null, location: null, conference_url: null,
  start: at(10, 5, 10), end: at(10, 5, 11), timezone: "Europe/Rome", all_day: false, recurrence_rule: "RRULE:FREQ=WEEKLY",
  status: "busy", etag: null, updated_at: null, sync_status: "synced", local_updated_at: null, remote_updated_at: null,
  occurrence_start: null, series_id: null, original_start: null,
};

describe("applyToSeries", () => {
  it("sposta la serie come l'occorrenza, in ora locale anche dopo il cambio d'ora", () => {
    // Occorrenza del 2 novembre (ora solare) spostata alle 11:30 di martedì 3, durata 30 minuti.
    const edited = { ...toNewEvent(series), title: "Standup lungo", start: at(11, 3, 11, 30), end: at(11, 3, 12) };
    const out = applyToSeries(series, at(11, 2, 10), edited, false);
    expect(out.start).toBe(at(10, 6, 11, 30));
    expect(out.end).toBe(at(10, 6, 12));
    expect(out.title).toBe("Standup lungo");
    expect(out.id).toBe("s");
  });

  it("da un'eccezione cambia solo i campi descrittivi e la regola", () => {
    const edited = { ...toNewEvent(series), title: "Nuovo", start: at(10, 13, 15), end: at(10, 13, 16), recurrence_rule: "RRULE:FREQ=DAILY" };
    const out = applyToSeries(series, at(10, 12, 10), edited, true);
    expect([out.start, out.end, out.title, out.recurrence_rule]).toEqual([series.start, series.end, "Nuovo", "RRULE:FREQ=DAILY"]);
  });
});
