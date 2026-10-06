import { describe, expect, it } from "vitest";
import { buildRecurrenceRule, parseRecurrence } from "./recurrence";

describe("buildRecurrenceRule", () => {
  it("scrive una RRULE settimanale con i giorni nell'ordine della settimana", () => {
    expect(buildRecurrenceRule({ freq: "weekly", byDay: ["WE", "MO"] }, null)).toBe("RRULE:FREQ=WEEKLY;BYDAY=MO,WE");
  });

  it("conserva le EXDATE esistenti quando cambiano i giorni", () => {
    const existing = "RRULE:FREQ=WEEKLY;BYDAY=MO\nEXDATE:20261012T080000Z\nEXDATE:20261019T080000Z";
    expect(buildRecurrenceRule({ freq: "weekly", byDay: ["MO", "FR"] }, existing)).toBe(
      "RRULE:FREQ=WEEKLY;BYDAY=MO,FR\nEXDATE:20261012T080000Z\nEXDATE:20261019T080000Z",
    );
  });

  it("riscrive in formato RRULE: una regola legacy senza prefisso", () => {
    expect(buildRecurrenceRule({ freq: "daily", byDay: [] }, "FREQ=DAILY")).toBe("RRULE:FREQ=DAILY");
  });

  it("mantiene INTERVAL e COUNT se la frequenza non cambia, ma li scarta se cambia", () => {
    const existing = "RRULE:FREQ=DAILY;INTERVAL=2;COUNT=5\nEXDATE:20261012T080000Z";
    expect(buildRecurrenceRule({ freq: "daily", byDay: [] }, existing)).toBe(existing);
    expect(buildRecurrenceRule({ freq: "monthly", byDay: [] }, existing)).toBe("RRULE:FREQ=MONTHLY\nEXDATE:20261012T080000Z");
  });

  it("azzera la ricorrenza con none e non tocca una regola custom", () => {
    expect(buildRecurrenceRule({ freq: "none", byDay: [] }, "RRULE:FREQ=DAILY")).toBeNull();
    expect(buildRecurrenceRule({ freq: "custom", byDay: [] }, "RRULE:FREQ=HOURLY")).toBe("RRULE:FREQ=HOURLY");
  });
});

describe("parseRecurrence", () => {
  it("legge frequenza e giorni, anche da una regola legacy", () => {
    expect(parseRecurrence("RRULE:FREQ=WEEKLY;BYDAY=TU,TH\nEXDATE:20261013T080000Z")).toEqual({ freq: "weekly", byDay: ["TU", "TH"] });
    expect(parseRecurrence("FREQ=MONTHLY")).toEqual({ freq: "monthly", byDay: [] });
    expect(parseRecurrence(null)).toEqual({ freq: "none", byDay: [] });
    expect(parseRecurrence("RRULE:FREQ=HOURLY").freq).toBe("custom");
  });
});
