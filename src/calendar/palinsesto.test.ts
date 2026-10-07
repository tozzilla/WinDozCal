import { describe, expect, it } from "vitest";
import type { Calendar, Event } from "@/types";
import { toIsoWithOffset } from "@/utils/date";
import { broadcastMarks } from "./palinsesto";

const at = (h: number, m = 0) => new Date(2026, 9, 7, h, m);
const cal = (id: string, account: string): Calendar => ({
  id, account_id: account, remote_id: id, name: id, color: "#000", visible: true, read_only: false,
});
const ev = (id: string, calendar: string, s: Date, e: Date): Event => ({
  id, calendar_id: calendar, remote_id: null, title: id, description: null, location: null, conference_url: null,
  start: toIsoWithOffset(s), end: toIsoWithOffset(e), timezone: "Europe/Rome", all_day: false, recurrence_rule: null,
  status: "busy", etag: null, updated_at: null, sync_status: "synced", local_updated_at: null, remote_updated_at: null,
  occurrence_start: null, series_id: null, original_start: null,
});

describe("broadcastMarks", () => {
  const calendars = [cal("riunioni", "google"), cal("commerciale", "google"), cal("cliente", "microsoft")];

  it("segna in onda, a seguire e il conflitto tra account diversi", () => {
    const events = [
      ev("revisione", "riunioni", at(13, 45), at(15, 15)),
      ev("call", "cliente", at(14, 30), at(16)),
      ev("sintesi", "riunioni", at(16, 15), at(17, 15)),
      ev("ieri", "riunioni", new Date(2026, 9, 6, 16), new Date(2026, 9, 6, 17)),
    ];
    const marks = broadcastMarks(events, calendars, at(14, 20));
    expect(Object.fromEntries(marks)).toEqual({ "revisione|": "onair", "call|": "conflict", "sintesi|": "next" });

    // Alle 14:40 la call è in onda insieme alla revisione; la sintesi diventa la prossima.
    expect(Object.fromEntries(broadcastMarks(events, calendars, at(14, 40)))).toEqual({
      "revisione|": "onair",
      "call|": "onair",
      "sintesi|": "next",
    });
  });

  it("non segnala come conflitto due eventi dello stesso account", () => {
    const events = [ev("a", "riunioni", at(9), at(10)), ev("b", "commerciale", at(9, 30), at(10, 30))];
    expect(broadcastMarks(events, calendars, at(8)).get("b|")).toBeUndefined();
    const cross = [ev("a", "riunioni", at(9), at(10)), ev("b", "cliente", at(9, 30), at(10, 30))];
    expect(broadcastMarks(cross, calendars, at(8)).get("b|")).toBe("conflict");
  });
});
