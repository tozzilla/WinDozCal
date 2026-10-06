import { describe, expect, it } from "vitest";
import { defaultEndTime, draftToFields, emptyDraft } from "./draft";

describe("ora di fine di default", () => {
  it("è un'ora dopo l'inizio", () => {
    expect(emptyDraft("c", { start: new Date(2026, 9, 6, 10), end: new Date(2026, 9, 6, 11) }).endTime).toBe("11:00");
  });

  it("alle 23:00 si limita alle 23:59 invece di finire prima dell'inizio", () => {
    const start = new Date(2026, 9, 6, 23);
    expect(defaultEndTime(start, new Date(start.getTime() + 3_600_000))).toBe("23:59");
    const draft = emptyDraft("c", { start, end: new Date(start.getTime() + 3_600_000) });
    expect(draft.endTime).toBe("23:59");
    const fields = draftToFields({ ...draft, title: "x" });
    expect(new Date(fields.end).getTime()).toBeGreaterThan(new Date(fields.start).getTime());
  });
});

describe("draftToFields: ricorrenza", () => {
  it("scrive la RRULE settimanale conservando le EXDATE della serie aperta", () => {
    const draft = {
      ...emptyDraft("c"),
      title: "Serie",
      recurrence: "weekly" as const,
      recurrenceDays: ["TU" as const, "TH" as const],
      existingRule: "RRULE:FREQ=WEEKLY;BYDAY=TU\nEXDATE:20261013T080000Z",
    };
    expect(draftToFields(draft).recurrence_rule).toBe("RRULE:FREQ=WEEKLY;BYDAY=TU,TH\nEXDATE:20261013T080000Z");
  });
});
