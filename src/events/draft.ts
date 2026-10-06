import type { Event, EventStatus, NewEvent } from "@/types";
import { addDays, localTimezone, toIsoWithOffset } from "@/utils/date";

export type RecurrencePreset = "none" | "daily" | "weekly" | "monthly" | "yearly";

const RRULE: Record<Exclude<RecurrencePreset, "none">, string> = {
  daily: "FREQ=DAILY",
  weekly: "FREQ=WEEKLY",
  monthly: "FREQ=MONTHLY",
  yearly: "FREQ=YEARLY",
};

/**
 * Stato del form dell'editor (PRD §7). `videoconference`, `attendees` e `reminderMinutes` sono
 * raccolti dalla UI ma non hanno ancora un campo nel contratto `Event`: non vengono inviati.
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
  videoconference: boolean;
  attendees: string;
  recurrence: RecurrencePreset;
  reminderMinutes: number | null;
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
    videoconference: false,
    attendees: "",
    recurrence: "none",
    reminderMinutes: 10,
    status: "busy",
  };
}

function presetFromRule(rule: string | null): RecurrencePreset {
  const found = Object.entries(RRULE).find(([, v]) => rule?.includes(v));
  return (found?.[0] as RecurrencePreset | undefined) ?? "none";
}

export function eventToDraft(e: Event): EventDraft {
  const start = new Date(e.start);
  const end = new Date(e.end);
  return {
    ...emptyDraft(e.calendar_id, { start, end }),
    title: e.title,
    allDay: e.all_day,
    location: e.location ?? "",
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
    start: toIsoWithOffset(start),
    end: toIsoWithOffset(end),
    timezone: localTimezone(),
    all_day: d.allDay,
    recurrence_rule: d.recurrence === "none" ? null : RRULE[d.recurrence],
    status: d.status,
  };
}
