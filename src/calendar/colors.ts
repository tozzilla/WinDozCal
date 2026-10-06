import type { Calendar } from "@/types";

export function calendarColor(calendars: Calendar[], calendarId: string): string {
  return calendars.find((c) => c.id === calendarId)?.color ?? "#64748b";
}
