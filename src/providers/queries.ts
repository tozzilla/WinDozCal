import { QueryClient, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Event, NewAttendee, NewEvent, NewReminder, Settings } from "@/types";
import { calendarService } from "./calendarService";

export const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 30_000, refetchOnWindowFocus: false } },
});

export const keys = {
  accounts: ["accounts"] as const,
  calendars: ["calendars"] as const,
  events: (start: string, end: string) => ["events", start, end] as const,
  detail: (eventId: string) => ["events", "detail", eventId] as const,
  settings: ["settings"] as const,
  search: (query: string) => ["events", "search", query] as const,
};

export const useAccounts = () => useQuery({ queryKey: keys.accounts, queryFn: () => calendarService.listAccounts() });

export const useCalendars = () => useQuery({ queryKey: keys.calendars, queryFn: () => calendarService.listCalendars() });

export const useEvents = (rangeStart: string, rangeEnd: string) =>
  useQuery({
    queryKey: keys.events(rangeStart, rangeEnd),
    queryFn: () => calendarService.listEvents(rangeStart, rangeEnd),
    placeholderData: (previous) => previous,
  });

/** Dettaglio (partecipanti e promemoria) di un evento esistente; `null` = nessuna query. */
export const useEventDetail = (eventId: string | null) =>
  useQuery({
    queryKey: keys.detail(eventId ?? ""),
    queryFn: () => calendarService.getEvent(eventId as string),
    enabled: eventId !== null,
    staleTime: 0,
  });

/** Lettura imperativa del dettaglio (es. da un evento del tray), con la stessa cache di `useEventDetail`. */
export const fetchEventDetail = (eventId: string) =>
  queryClient.fetchQuery({ queryKey: keys.detail(eventId), queryFn: () => calendarService.getEvent(eventId), staleTime: 0 });

export const useSettings = () => useQuery({ queryKey: keys.settings, queryFn: () => calendarService.getSettings() });

export function useUpdateSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (settings: Settings) => calendarService.updateSettings(settings),
    onSuccess: (saved) => qc.setQueryData(keys.settings, saved),
  });
}

export const useSearchEvents = (query: string) =>
  useQuery({
    queryKey: keys.search(query),
    queryFn: () => calendarService.searchEvents(query),
    enabled: query.trim().length > 0,
  });

/** Le mutazioni invalidano le query di eventi; la visibilità invalida anche i calendari. */
function useInvalidate() {
  const qc = useQueryClient();
  return (alsoCalendars = false) => {
    void qc.invalidateQueries({ queryKey: ["events"] });
    if (alsoCalendars) void qc.invalidateQueries({ queryKey: keys.calendars });
  };
}

export function useCreateLocalAccount() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => calendarService.createLocalAccount(name),
    onSuccess: () => void qc.invalidateQueries(),
  });
}

export function useCreateCalendar() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (v: { accountId: string; name: string; color: string }) => calendarService.createCalendar(v.accountId, v.name, v.color),
    onSuccess: () => void qc.invalidateQueries({ queryKey: keys.calendars }),
  });
}

export function useSetCalendarVisibility() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: ({ calendarId, visible }: { calendarId: string; visible: boolean }) =>
      calendarService.setCalendarVisibility(calendarId, visible),
    onSuccess: () => invalidate(true),
  });
}

export function useCreateEvent() {
  const invalidate = useInvalidate();
  return useMutation({ mutationFn: (v: { event: NewEvent; attendees: NewAttendee[]; reminders: NewReminder[] }) =>
      calendarService.createEvent(v.event, v.attendees, v.reminders), onSuccess: () => invalidate() });
}

export function useUpdateEvent() {
  const invalidate = useInvalidate();
  return useMutation({ mutationFn: (v: { event: Event; attendees: NewAttendee[]; reminders: NewReminder[] }) =>
      calendarService.updateEvent(v.event, v.attendees, v.reminders), onSuccess: () => invalidate() });
}

export function useDeleteEvent() {
  const invalidate = useInvalidate();
  return useMutation({ mutationFn: (eventId: string) => calendarService.deleteEvent(eventId), onSuccess: () => invalidate() });
}

export function useDeleteOccurrence() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: (v: { eventId: string; occurrenceStart: string }) => calendarService.deleteOccurrence(v.eventId, v.occurrenceStart),
    onSuccess: () => invalidate(),
  });
}

/**
 * Sposta o ridimensiona un evento non ricorrente con aggiornamento ottimistico delle liste di eventi in cache;
 * se il backend rifiuta, la cache torna com'era (il chiamante mostra l'errore).
 * L'update rinvia partecipanti e promemoria letti con `get_event`, perché gli array sostituiscono quelli esistenti.
 */
export function useChangeEventTime() {
  const qc = useQueryClient();
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async (v: { event: Event; start: string; end: string }) => {
      const detail = await calendarService.getEvent(v.event.id);
      return calendarService.updateEvent(
        { ...detail.event, start: v.start, end: v.end },
        detail.attendees.map((a) => ({ email: a.email, name: a.name })),
        detail.reminders.map((r) => ({ minutes_before: r.minutes_before, type: r.type })),
      );
    },
    onMutate: async (v) => {
      await qc.cancelQueries({ queryKey: ["events"] });
      const snapshots = qc.getQueriesData({ queryKey: ["events"] });
      qc.setQueriesData({ queryKey: ["events"] }, (old: unknown) =>
        Array.isArray(old)
          ? (old as Event[]).map((e) =>
              e.id === v.event.id && e.occurrence_start === v.event.occurrence_start ? { ...e, start: v.start, end: v.end } : e,
            )
          : old,
      );
      return { snapshots };
    },
    onError: (_err, _v, ctx) => ctx?.snapshots.forEach(([key, data]) => qc.setQueryData(key, data)),
    onSettled: () => invalidate(),
  });
}

export function useSyncNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (accountId?: string) => calendarService.syncNow(accountId),
    onSuccess: () => void qc.invalidateQueries(),
  });
}

export const openLogFolder = () => calendarService.openLogFolder();
