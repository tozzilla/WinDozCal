import { QueryClient, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { applyToSeries, type SeriesScope } from "@/events/seriesEdit";
import type { Event, NewAttendee, NewEvent, NewReminder, Settings } from "@/types";
import type { OccurrenceRef } from "@/utils/events";
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

/** Collegamento, riconnessione e scollegamento degli account: ricaricano tutto (account, calendari, eventi). */
export function useConnectMicrosoft() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: () => calendarService.connectMicrosoft(), onSuccess: () => void qc.invalidateQueries() });
}

export function useReconnectAccount() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (accountId: string) => calendarService.reconnectAccount(accountId), onSuccess: () => void qc.invalidateQueries() });
}

export function useDisconnectAccount() {
  const qc = useQueryClient();
  return useMutation({ mutationFn: (accountId: string) => calendarService.disconnectAccount(accountId), onSuccess: () => void qc.invalidateQueries() });
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

export function useSetCalendarColor() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: ({ calendarId, color }: { calendarId: string; color: string }) => calendarService.setCalendarColor(calendarId, color),
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

/**
 * Salva una modifica fatta su un'occorrenza di serie con la portata scelta (PRD §10, ADR 013):
 * - `this`: eccezione (`update_occurrence`), o `update_event` se l'occorrenza è già un'eccezione (`exception`);
 * - `following`: `split_series` con i campi e la regola modificati;
 * - `all`: `update_event` sulla serie, spostata come l'occorrenza (vedi `applyToSeries`).
 */
export function useSaveOccurrence() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: async (v: {
      ref: OccurrenceRef;
      scope: SeriesScope;
      fields: NewEvent;
      attendees: NewAttendee[];
      reminders: NewReminder[];
      exception?: Event;
    }) => {
      const single = { ...v.fields, recurrence_rule: null };
      if (v.scope === "this") {
        return v.exception
          ? calendarService.updateEvent({ ...v.exception, ...single }, v.attendees, v.reminders)
          : calendarService.updateOccurrence(v.ref.seriesId, v.ref.occurrenceStart, single, v.attendees, v.reminders);
      }
      if (v.scope === "following") {
        return calendarService.splitSeries(v.ref.seriesId, v.ref.occurrenceStart, v.fields, v.attendees, v.reminders);
      }
      const series = await calendarService.getEvent(v.ref.seriesId);
      return calendarService.updateEvent(applyToSeries(series.event, v.ref.occurrenceStart, v.fields, v.ref.isException), v.attendees, v.reminders);
    },
    onSuccess: () => invalidate(),
  });
}

/** Cancellazione di un'occorrenza con la portata scelta. */
export function useDeleteOccurrenceScoped() {
  const invalidate = useInvalidate();
  return useMutation({
    mutationFn: ({ ref, scope }: { ref: OccurrenceRef; scope: SeriesScope }) =>
      scope === "this"
        ? calendarService.deleteOccurrence(ref.seriesId, ref.occurrenceStart)
        : scope === "following"
          ? calendarService.truncateSeries(ref.seriesId, ref.occurrenceStart)
          : calendarService.deleteEvent(ref.seriesId),
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

export const useAppVersion = () => useQuery({ queryKey: ["app-version"], queryFn: () => calendarService.appVersion(), staleTime: Infinity });

/** Controllo aggiornamenti: all'avvio (con un attimo di ritardo, PRD §36) e poi ogni 6 ore; mai in errore bloccante. */
export const useUpdateCheck = () =>
  useQuery({
    queryKey: ["update"],
    queryFn: () => calendarService.checkUpdate(),
    staleTime: 6 * 60 * 60_000,
    refetchInterval: 6 * 60 * 60_000,
    retry: false,
  });

export function useInstallUpdate() {
  return useMutation({ mutationFn: () => calendarService.installUpdate() });
}
