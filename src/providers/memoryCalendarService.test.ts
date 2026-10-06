import { describe, expect, it } from "vitest";
import type { NewEvent } from "@/types";
import { createMemoryCalendarService } from "./memoryCalendarService";

const event = (calendar_id: string): NewEvent => ({
  calendar_id, title: "Prova", description: null, location: null, conference_url: null,
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

    const { event: created } = await svc.createEvent(event(cal.id), [], []);
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

  describe("partecipanti e promemoria", () => {
    const setup = async () => {
      const svc = createMemoryCalendarService();
      await svc.createLocalAccount("Questo computer");
      const [cal] = await svc.listCalendars();
      return { svc, cal };
    };

    it("get_event restituisce ciò che create_event ha salvato", async () => {
      const { svc, cal } = await setup();
      const created = await svc.createEvent(
        { ...event(cal.id), conference_url: "https://meet.example.com/abc" },
        [{ email: "anna@example.com", name: "Anna" }, { email: "bruno@example.com", name: null }],
        [{ minutes_before: 10, type: "popup" }],
      );
      const detail = await svc.getEvent(created.event.id);
      expect(detail.event.conference_url).toBe("https://meet.example.com/abc");
      expect(detail.attendees.map((a) => [a.email, a.name, a.status])).toEqual([
        ["anna@example.com", "Anna", "needs_action"],
        ["bruno@example.com", null, "needs_action"],
      ]);
      expect(detail.reminders.map((r) => [r.minutes_before, r.type])).toEqual([[10, "popup"]]);
    });

    it("update_event sostituisce gli array invece di accodarli", async () => {
      const { svc, cal } = await setup();
      const { event: created } = await svc.createEvent(
        event(cal.id),
        [{ email: "anna@example.com", name: null }],
        [{ minutes_before: 10, type: "popup" }],
      );
      await svc.updateEvent(created, [{ email: "carla@example.com", name: null }], [{ minutes_before: 60, type: "email" }]);
      const detail = await svc.getEvent(created.id);
      expect(detail.attendees.map((a) => a.email)).toEqual(["carla@example.com"]);
      expect(detail.reminders.map((r) => [r.minutes_before, r.type])).toEqual([[60, "email"]]);

      await svc.updateEvent(created, [], []);
      expect((await svc.getEvent(created.id)).attendees).toEqual([]);
    });

    it("rifiuta email non valide, duplicati e minuti fuori intervallo senza salvare nulla", async () => {
      const { svc, cal } = await setup();
      await expect(svc.createEvent(event(cal.id), [{ email: "non-una-email", name: null }], [])).rejects.toThrow(/email non valido/);
      await expect(
        svc.createEvent(event(cal.id), [{ email: "a@x.it", name: null }, { email: "A@x.it", name: null }], []),
      ).rejects.toThrow(/duplicato/);
      await expect(svc.createEvent(event(cal.id), [], [{ minutes_before: -5, type: "popup" }])).rejects.toThrow(/Promemoria/);
      await expect(svc.createEvent(event(cal.id), [], [{ minutes_before: 1.5, type: "popup" }])).rejects.toThrow(/Promemoria/);
      expect(await svc.listEvents("2026-10-05T00:00:00+02:00", "2026-10-12T00:00:00+02:00")).toEqual([]);
    });
  });

  it("espone le impostazioni di default e le salva con un roundtrip", async () => {
    const svc = createMemoryCalendarService();
    expect(await svc.getSettings()).toEqual({ start_on_login: false, start_minimized: false, close_to_tray: true });

    const saved = await svc.updateSettings({ start_on_login: true, start_minimized: true, close_to_tray: false });
    expect(saved).toEqual({ start_on_login: true, start_minimized: true, close_to_tray: false });
    expect(await svc.getSettings()).toEqual(saved);
  });

  describe("serie ricorrenti", () => {
    const range = ["2026-10-05T00:00:00+02:00", "2026-10-26T00:00:00+01:00"] as const;
    const day = (iso: string) => new Date(iso).getDate();

    it("list_events espande la serie con occurrence_start; delete_occurrence aggiunge una EXDATE senza toccare le altre", async () => {
      const svc = createMemoryCalendarService();
      await svc.createLocalAccount("Questo computer");
      const [cal] = await svc.listCalendars();
      const { event: series } = await svc.createEvent({ ...event(cal.id), recurrence_rule: "RRULE:FREQ=WEEKLY;BYDAY=TU" }, [], []);

      const before = await svc.listEvents(...range);
      expect(before.map((e) => day(e.start))).toEqual([6, 13, 20]);
      expect(before.every((e) => e.id === series.id && e.occurrence_start === e.start)).toBe(true);

      await svc.deleteOccurrence(series.id, before[1].occurrence_start as string);
      const after = await svc.listEvents(...range);
      expect(after.map((e) => day(e.start))).toEqual([6, 20]);

      const detail = await svc.getEvent(series.id);
      expect(detail.event.occurrence_start).toBeNull();
      expect(detail.event.recurrence_rule).toMatch(/^RRULE:FREQ=WEEKLY;BYDAY=TU\nEXDATE:\d{8}T\d{6}Z$/);
    });
  });
});
