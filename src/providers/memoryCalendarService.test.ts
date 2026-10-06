import { describe, expect, it } from "vitest";
import type { NewEvent } from "@/types";
import { createMemoryCalendarService } from "./memoryCalendarService";

const event = (calendar_id: string): NewEvent => ({
  calendar_id, title: "Prova", description: null, location: null,
  start: "2026-10-06T09:00:00+02:00", end: "2026-10-06T10:00:00+02:00", timezone: "Europe/Rome",
  all_day: false, recurrence_rule: null, status: "busy",
});

describe("memoryCalendarService", () => {
  it("parte senza account, crea l'account locale in modo idempotente con il calendario Personale", async () => {
    const svc = createMemoryCalendarService();
    expect(await svc.listAccounts()).toEqual([]);

    const first = await svc.createLocalAccount("Questo computer");
    const second = await svc.createLocalAccount("Altro nome");
    expect(second.id).toBe(first.id);
    expect(first.provider).toBe("local");
    expect(await svc.listAccounts()).toHaveLength(1);

    const calendars = await svc.listCalendars();
    expect(calendars.map((c) => c.name)).toEqual(["Personale"]);
  });

  it("mostra in list_events l'evento creato, già synced, e lo nasconde con il calendario", async () => {
    const svc = createMemoryCalendarService();
    await svc.createLocalAccount("Questo computer");
    const [cal] = await svc.listCalendars();

    const created = await svc.createEvent(event(cal.id));
    expect(created.sync_status).toBe("synced");
    const range = ["2026-10-05T00:00:00+02:00", "2026-10-12T00:00:00+02:00"] as const;
    expect((await svc.listEvents(...range)).map((e) => e.id)).toEqual([created.id]);

    await svc.setCalendarVisibility(cal.id, false);
    expect(await svc.listEvents(...range)).toEqual([]);
  });

  it("rifiuta create_calendar su account non locali", async () => {
    const svc = createMemoryCalendarService(true);
    await expect(svc.createCalendar("acc-google", "X", "#000000")).rejects.toThrow();
  });
});
