import type { Account, Attendee, Calendar, Event, EventDetail, NewAttendee, NewEvent, NewReminder, Reminder, Settings } from "@/types";
import { validateAttendees, validateReminders } from "@/utils/validation";
import { expandEvent, exdateValue } from "./memoryRecurrence";
import { addDays, localTimezone, startOfWeek, toIsoWithOffset } from "@/utils/date";
import type { CalendarService } from "./calendarService";

const DEFAULT_COLOR = "#2563eb";

// Fallback in-memory per `npm run dev` nel browser: si comporta come il backend (docs/CONTRACT.md).
// Parte vuoto, così si vede la WelcomeScreen; i dati d'esempio si attivano con `?demo` nell'URL.

const demoAccounts = (): Account[] => [
  { id: "acc-google", provider: "google", name: "Google", email: "demo@example.com", sync_status: "idle", last_sync: null },
  { id: "acc-work", provider: "caldav", name: "Work", email: "demo@example.com", sync_status: "idle", last_sync: null },
  { id: "acc-ms", provider: "microsoft", name: "Microsoft", email: "demo@contoso.example", sync_status: "idle", last_sync: null },
];

const demoCalendars = (): Calendar[] => [
  { id: "cal-personale", account_id: "acc-google", remote_id: "personale", name: "Personale", color: "#2563eb", visible: true, read_only: false },
  { id: "cal-famiglia", account_id: "acc-google", remote_id: "famiglia", name: "Famiglia", color: "#16a34a", visible: true, read_only: false },
  { id: "cal-riunioni", account_id: "acc-work", remote_id: "riunioni", name: "Riunioni", color: "#9333ea", visible: true, read_only: false },
  { id: "cal-commerciale", account_id: "acc-work", remote_id: "commerciale", name: "Commerciale", color: "#ea580c", visible: true, read_only: false },
  { id: "cal-cliente", account_id: "acc-ms", remote_id: "cliente-xyz", name: "Cliente XYZ", color: "#0891b2", visible: true, read_only: true },
];

function seedEvents(): Event[] {
  const monday = startOfWeek(new Date());
  const make = (
    id: string,
    calendar_id: string,
    title: string,
    day: number,
    h: number,
    m: number,
    minutes: number,
    extra: Partial<Event> = {},
  ): Event => {
    const start = addDays(monday, day);
    start.setHours(h, m, 0, 0);
    const end = new Date(start.getTime() + minutes * 60_000);
    return {
      id, calendar_id, remote_id: null, title, description: null, location: null, conference_url: null, occurrence_start: null,
      start: toIsoWithOffset(start), end: toIsoWithOffset(end), timezone: localTimezone(),
      all_day: false, recurrence_rule: null, status: "busy", etag: null, updated_at: null,
      sync_status: "synced", local_updated_at: null, remote_updated_at: null, ...extra,
    };
  };
  return [
    make("ev-1", "cal-riunioni", "Riunione commerciale", 0, 9, 30, 60),
    make("ev-2", "cal-commerciale", "Call cliente", 1, 11, 0, 45, { location: "Videoconferenza" }),
    make("ev-3", "cal-riunioni", "Revisione progetto", 1, 15, 30, 90),
    make("ev-4", "cal-cliente", "Presentazione", 2, 10, 0, 60),
    make("ev-5", "cal-personale", "Dentista", 3, 14, 30, 60),
    make("ev-6", "cal-famiglia", "Cena di famiglia", 4, 20, 0, 120),
    make("ev-7", "cal-personale", "Weekend fuori porta", 5, 0, 0, 48 * 60, { all_day: true }),
  ];
}


export function createMemoryCalendarService(demo = false): CalendarService {
  const accounts: Account[] = demo ? demoAccounts() : [];
  const calendars: Calendar[] = demo ? demoCalendars() : [];
  let events: Event[] = demo ? seedEvents() : [];
  const attendeesByEvent = new Map<string, Attendee[]>();
  const remindersByEvent = new Map<string, Reminder[]>();
  let settings: Settings = { start_on_login: false, start_minimized: false, close_to_tray: true };
  let nextId = 1;
  const newId = (prefix: string) => `${prefix}-${nextId++}`;
  const isLocal = (calendarId: string) =>
    accounts.find((a) => a.id === calendars.find((c) => c.id === calendarId)?.account_id)?.provider === "local";

  const validate = (attendees: NewAttendee[], reminders: NewReminder[]) => {
    const errors = [...validateAttendees(attendees), ...validateReminders(reminders)];
    if (errors.length > 0) throw new Error(errors.join(" "));
  };
  // Gli array sostituiscono interamente quelli esistenti, come nel backend.
  const storeExtras = (eventId: string, attendees: NewAttendee[], reminders: NewReminder[]) => {
    attendeesByEvent.set(
      eventId,
      attendees.map((a) => ({ id: newId("att"), event_id: eventId, email: a.email.trim(), name: a.name?.trim() || null, status: "needs_action" })),
    );
    remindersByEvent.set(eventId, reminders.map((r) => ({ id: newId("rem"), event_id: eventId, ...r })));
  };
  const detailOf = (event: Event): EventDetail =>
    structuredClone({ event, attendees: attendeesByEvent.get(event.id) ?? [], reminders: remindersByEvent.get(event.id) ?? [] });

  return {
    async listAccounts() {
      return structuredClone(accounts);
    },
    async createLocalAccount(name) {
      let account = accounts.find((a) => a.provider === "local");
      if (!account) {
        account = { id: newId("acc-local"), provider: "local", name, email: "", sync_status: "synced", last_sync: null };
        accounts.push(account);
        calendars.push({
          id: newId("cal"), account_id: account.id, remote_id: "", name: "Personale",
          color: DEFAULT_COLOR, visible: true, read_only: false,
        });
      }
      return structuredClone(account);
    },
    async createCalendar(accountId, name, color) {
      const account = accounts.find((a) => a.id === accountId);
      if (account?.provider !== "local") throw new Error("I calendari si possono creare solo su account locali");
      const cal: Calendar = { id: newId("cal"), account_id: accountId, remote_id: "", name, color, visible: true, read_only: false };
      calendars.push(cal);
      return structuredClone(cal);
    },
    async listCalendars() {
      return structuredClone(calendars);
    },
    async setCalendarVisibility(calendarId, visible) {
      const cal = calendars.find((c) => c.id === calendarId);
      if (cal) cal.visible = visible;
    },
    async listEvents(rangeStart, rangeEnd) {
      const from = new Date(rangeStart).getTime();
      const to = new Date(rangeEnd).getTime();
      const visible = new Set(calendars.filter((c) => c.visible).map((c) => c.id));
      return structuredClone(
        events
          .filter((e) => visible.has(e.calendar_id))
          .flatMap((e) => expandEvent(e, new Date(from), new Date(to)))
          .sort((a, b) => a.start.localeCompare(b.start)),
      );
    },
    async getEvent(eventId) {
      const event = events.find((e) => e.id === eventId);
      if (!event) throw new Error(`Evento non trovato: ${eventId}`);
      return detailOf(event);
    },
    async createEvent(input: NewEvent, attendees: NewAttendee[], reminders: NewReminder[]) {
      validate(attendees, reminders);
      const created: Event = {
        ...input,
        id: newId("ev"), remote_id: null, etag: null, updated_at: null, occurrence_start: null,
        // Come il backend: gli eventi locali restano "synced", gli altri entrano in coda.
        sync_status: isLocal(input.calendar_id) ? "synced" : "pending_create",
        local_updated_at: toIsoWithOffset(new Date()), remote_updated_at: null,
      };
      events.push(created);
      storeExtras(created.id, attendees, reminders);
      return detailOf(created);
    },
    async updateEvent(event, attendees, reminders) {
      validate(attendees, reminders);
      const updated: Event = {
        ...event,
        occurrence_start: null,
        sync_status: isLocal(event.calendar_id) ? "synced" : "pending_update",
        local_updated_at: toIsoWithOffset(new Date()),
      };
      events = events.map((e) => (e.id === event.id ? updated : e));
      storeExtras(updated.id, attendees, reminders);
      return detailOf(updated);
    },
    async deleteEvent(eventId) {
      events = events.filter((e) => e.id !== eventId);
      attendeesByEvent.delete(eventId);
      remindersByEvent.delete(eventId);
    },
    async deleteOccurrence(eventId, occurrenceStart) {
      const event = events.find((e) => e.id === eventId);
      if (!event?.recurrence_rule) throw new Error("L'evento non è una serie ricorrente");
      const line = `EXDATE:${exdateValue(occurrenceStart, event.all_day)}`;
      event.recurrence_rule = `${event.recurrence_rule}\n${line}`;
      event.local_updated_at = toIsoWithOffset(new Date());
      if (!isLocal(event.calendar_id)) event.sync_status = "pending_update";
    },
    async searchEvents(query) {
      const q = query.trim().toLowerCase();
      if (!q) return [];
      return structuredClone(events.filter((e) => `${e.title} ${e.location ?? ""} ${e.description ?? ""}`.toLowerCase().includes(q)));
    },
    async syncNow() {},
    async openLogFolder() {},
    async getSettings() {
      return { ...settings };
    },
    async updateSettings(next) {
      settings = { ...next };
      return { ...settings };
    },
  };
}
