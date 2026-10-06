import type { Attendee, Event, EventStatus, NewAttendee, NewEvent, NewReminder, Reminder } from "@/types";
import { addDays, localTimezone, toIsoWithOffset } from "@/utils/date";
import { isValidConferenceUrl, validateAttendees, validateReminders } from "@/utils/validation";

export type RecurrencePreset = "none" | "daily" | "weekly" | "monthly" | "yearly";

const RRULE: Record<Exclude<RecurrencePreset, "none">, string> = {
  daily: "FREQ=DAILY",
  weekly: "FREQ=WEEKLY",
  monthly: "FREQ=MONTHLY",
  yearly: "FREQ=YEARLY",
};

/**
 * Stato del form dell'editor (PRD §7). Partecipanti e promemoria viaggiano a parte rispetto
 * a `Event` (`NewAttendee[]`, `NewReminder[]`), come nei comandi `create_event`/`update_event`.
 */
export interface EventDraft {
  title: string;
  date: string; // YYYY-MM-DD
  startTime: string; // HH:MM
  endTime: string; // HH:MM
  allDay: boolean;
  calendarId: string;
  location: string;
  description: string;
  conferenceUrl: string;
  attendees: NewAttendee[];
  recurrence: RecurrencePreset;
  reminders: NewReminder[];
  status: EventStatus;
}

const pad = (n: number) => String(n).padStart(2, "0");
export const toDateInput = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
export const toTimeInput = (d: Date) => `${pad(d.getHours())}:${pad(d.getMinutes())}`;

export function emptyDraft(calendarId: string, slot?: { start: Date; end: Date } | null): EventDraft {
  const start = slot?.start ?? new Date(new Date().setMinutes(0, 0, 0));
  const end = slot?.end ?? new Date(start.getTime() + 60 * 60_000);
  return {
    title: "",
    date: toDateInput(start),
    startTime: toTimeInput(start),
    endTime: toTimeInput(end),
    allDay: false,
    calendarId,
    location: "",
    description: "",
    conferenceUrl: "",
    attendees: [],
    recurrence: "none",
    reminders: [],
    status: "busy",
  };
}

function presetFromRule(rule: string | null): RecurrencePreset {
  const found = Object.entries(RRULE).find(([, v]) => rule?.includes(v));
  return (found?.[0] as RecurrencePreset | undefined) ?? "none";
}

export function eventToDraft(e: Event, attendees: Attendee[] = [], reminders: Reminder[] = []): EventDraft {
  const start = new Date(e.start);
  const end = new Date(e.end);
  return {
    ...emptyDraft(e.calendar_id, { start, end }),
    title: e.title,
    allDay: e.all_day,
    location: e.location ?? "",
    conferenceUrl: e.conference_url ?? "",
    attendees: attendees.map((a) => ({ email: a.email, name: a.name })),
    reminders: reminders.map((r) => ({ minutes_before: r.minutes_before, type: r.type })),
    description: e.description ?? "",
    recurrence: presetFromRule(e.recurrence_rule),
    status: e.status,
  };
}

/** Converte il form nei soli campi del contratto. */
export function draftToFields(d: EventDraft): NewEvent {
  const [y, m, day] = d.date.split("-").map(Number);
  const [sh, sm] = d.startTime.split(":").map(Number);
  const [eh, em] = d.endTime.split(":").map(Number);
  const start = d.allDay ? new Date(y, m - 1, day) : new Date(y, m - 1, day, sh, sm);
  let end = d.allDay ? addDays(start, 1) : new Date(y, m - 1, day, eh, em);
  if (end <= start) end = new Date(start.getTime() + 30 * 60_000);
  return {
    calendar_id: d.calendarId,
    title: d.title.trim(),
    description: d.description.trim() || null,
    location: d.location.trim() || null,
    conference_url: d.conferenceUrl.trim() || null,
    start: toIsoWithOffset(start),
    end: toIsoWithOffset(end),
    timezone: localTimezone(),
    all_day: d.allDay,
    recurrence_rule: d.recurrence === "none" ? null : RRULE[d.recurrence],
    status: d.status,
  };
}

/** Messaggi di validazione da mostrare prima dell'invio; lista vuota = form valido. */
export function validateDraft(d: EventDraft): string[] {
  const errors: string[] = [];
  if (!d.title.trim()) errors.push("Il titolo è obbligatorio.");
  if (!d.calendarId) errors.push("Scegli un calendario.");
  if (d.conferenceUrl.trim() && !isValidConferenceUrl(d.conferenceUrl)) {
    errors.push("Il link della videoconferenza deve iniziare con http:// o https://.");
  }
  return [...errors, ...validateAttendees(d.attendees), ...validateReminders(d.reminders)];
}
