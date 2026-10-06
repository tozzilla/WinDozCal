import type { NewAttendee, NewReminder } from "@/types";

/** Quattro settimane in minuti: il massimo accettato anche da Google Calendar. */
export const MAX_REMINDER_MINUTES = 40_320;

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export const isValidEmail = (email: string) => EMAIL.test(email.trim());

/** Solo http/https: il link della videoconferenza viene aperto fuori dall'app. */
export function isValidConferenceUrl(value: string): boolean {
  try {
    const { protocol } = new URL(value.trim());
    return protocol === "https:" || protocol === "http:";
  } catch {
    return false;
  }
}

export function validateAttendees(attendees: NewAttendee[]): string[] {
  const errors: string[] = [];
  const seen = new Set<string>();
  for (const a of attendees) {
    const email = a.email.trim();
    if (!isValidEmail(email)) errors.push(`Indirizzo email non valido: "${email}".`);
    else if (seen.has(email.toLowerCase())) errors.push(`Partecipante duplicato: ${email}.`);
    seen.add(email.toLowerCase());
  }
  return errors;
}

export function validateReminders(reminders: NewReminder[]): string[] {
  const errors: string[] = [];
  for (const r of reminders) {
    if (!Number.isInteger(r.minutes_before) || r.minutes_before < 0 || r.minutes_before > MAX_REMINDER_MINUTES) {
      errors.push(`Promemoria non valido: i minuti devono essere un intero tra 0 e ${MAX_REMINDER_MINUTES}.`);
    }
  }
  return errors;
}
