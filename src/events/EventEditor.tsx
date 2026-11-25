import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { dateFormat, errorText, useT } from "@/i18n";
import { openExternal } from "@/providers/externalLinks";
import {
  usePreferences,
  useAccounts,
  useCalendars,
  useCreateEvent,
  useDeleteEvent,
  useDeleteOccurrenceScoped,
  useDiscardFailedEvent,
  useEventDetail,
  useMoveEvent,
  useRetryFailedEvents,
  useSaveOccurrence,
  useUpdateEvent,
} from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { occurrenceRef } from "@/utils/events";
import { isValidConferenceUrl } from "@/utils/validation";
import { AppearanceField } from "./AppearanceField";
import { AttendeesField } from "./AttendeesField";
import { moveBlockReason } from "./move";
import { draftToFields, emptyDraft, eventToDraft, pickCalendar, toDateInput, toTimeInput, validateDraft, type EventDraft } from "./draft";
import { weekdayOf, WEEKDAYS, type RecurrenceFreq, type Weekday } from "./recurrence";
import { RemindersField } from "./RemindersField";
import { ScopeChoice } from "./ScopeChoice";
import type { SeriesScope } from "./seriesEdit";

/** Nome breve del giorno nella lingua attiva ("Lun", "Mon"); 1 gennaio 2024 era un lunedì. */
function weekdayLabel(d: Weekday): string {
  const name = dateFormat({ weekday: "short" }).format(new Date(2024, 0, 1 + WEEKDAYS.indexOf(d)));
  return name.charAt(0).toUpperCase() + name.slice(1);
}

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-[11px] font-bold tracking-[0.1em] text-muted-foreground uppercase">{label}</span>
      {children}
    </label>
  );
}

/** Editor evento minimo (PRD §7). Esc lo chiude (gestito da useKeyboardShortcuts). */
export function EventEditor() {
  const t = useT();
  const { open, openedAt, event, slot, preset, close } = useEditorStore();
  const { data: calendars = [] } = useCalendars();
  const { data: accounts = [] } = useAccounts();
  const settings = usePreferences();
  const create = useCreateEvent();
  const update = useUpdateEvent();
  const move = useMoveEvent();
  const retryFailed = useRetryFailedEvents();
  const discardFailed = useDiscardFailedEvent();
  const remove = useDeleteEvent();
  const removeScoped = useDeleteOccurrenceScoped();
  const saveOccurrence = useSaveOccurrence();
  const [confirmDelete, setConfirmDelete] = useState(false);
  /** Salvataggio di un'occorrenza in attesa della scelta della portata. */
  const [choosingScope, setChoosingScope] = useState(false);
  const initialised = useRef(false);
  // Occorrenza di una serie (espansa o eccezione): il dettaglio è quello della serie o dell'eccezione;
  // per un'eccezione serve anche la serie, da cui vengono la regola e "tutta la serie".
  const ref = event ? occurrenceRef(event) : null;
  const detail = useEventDetail(open && event ? event.id : null);
  const seriesDetail = useEventDetail(open && ref?.isException ? ref.seriesId : null);
  const [draft, setDraft] = useState<EventDraft | null>(null);
  const [errors, setErrors] = useState<string[]>([]);

  const writable = calendars.filter((c) => !c.read_only);

  // Il draft si inizializza una sola volta per apertura, con dati freschi: per un evento esistente si attende
  // il dettaglio (la serie intera, anche se si è cliccata un'occorrenza); i refetch successivi non devono
  // azzerare ciò che l'utente sta scrivendo.
  useEffect(() => {
    if (!open) {
      initialised.current = false;
      setDraft(null);
      setErrors([]);
      setConfirmDelete(false);
      setChoosingScope(false);
      return;
    }
    if (initialised.current) return;
    if (!event) {
      initialised.current = true;
      const base = emptyDraft(pickCalendar(calendars, settings.default_calendar_id)?.id ?? "", slot, settings.default_duration);
      setDraft(preset ? { ...base, title: preset.title, allDay: preset.allDay } : base);
    } else if (detail.data && !detail.isFetching && (!ref?.isException || (seriesDetail.data && !seriesDetail.isFetching))) {
      initialised.current = true;
      let d = eventToDraft(detail.data.event, detail.data.attendees, detail.data.reminders);
      if (ref && !ref.isException) {
        // Occorrenza espansa: data e orari sono quelli dell'occorrenza cliccata, il resto della serie.
        const start = new Date(event.start);
        d = { ...d, date: toDateInput(start), startTime: toTimeInput(start), endTime: toTimeInput(new Date(event.end)) };
      } else if (ref?.isException && seriesDetail.data) {
        const rule = eventToDraft(seriesDetail.data.event);
        d = { ...d, recurrence: rule.recurrence, recurrenceDays: rule.recurrenceDays, existingRule: rule.existingRule };
      }
      setDraft(d);
    }
  }, [open, event, slot, preset, detail.data, detail.isFetching, seriesDetail.data, seriesDetail.isFetching, writable.length]);

  // Chiude solo con un clic sul fondo; si ignora quello che arriva subito dopo l'apertura (secondo clic di un doppio clic).
  const onBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget && Date.now() - openedAt > 500) close();
  };

  if (open && event && !draft) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-rail/45 p-4" onClick={onBackdropClick}>
        <div className="rounded-lg border bg-popover p-5 text-sm shadow-[0_18px_48px_rgb(16_20_42/0.28)]">
          {detail.isError ? (
            <>
              <p className="mb-3 text-destructive">{t("events.editor.loadFailed", { error: errorText(detail.error) })}</p>
              <Button variant="outline" onClick={close}>
                {t("events.editor.close")}
              </Button>
            </>
          ) : (
            t("events.editor.loading")
          )}
        </div>
      </div>
    );
  }

  if (!open || !draft) return null;

  const set = <K extends keyof EventDraft>(key: K, value: EventDraft[K]) => setDraft({ ...draft, [key]: value });

  // Cambio di calendario di un evento esistente (ADR 020): copia nel nuovo calendario e cancellazione
  // dell'originale; per una serie si sposta sempre la serie intera.
  const currentCalendarId = detail.data?.event.calendar_id;
  // Evento di un calendario di sola lettura (es. calendari condivisi o festività): si consulta soltanto.
  const sourceCalendar = calendars.find((c) => c.id === currentCalendarId);
  const readOnly = !!event && !!sourceCalendar?.read_only;
  // Modifica rifiutata dal server (ADR 021): l'editor offre Riprova o Ripristina dal server.
  const failedId = detail.data?.event.sync_status === "error" ? detail.data.event.id : null;
  const moving = !!event && !!currentCalendarId && draft.calendarId !== currentCalendarId;
  // CalDAV (ADR 023): l'app non scrive mai i partecipanti, perché i server con scheduling implicito
  // invierebbero gli inviti; quelli che arrivano dal server si vedono in sola lettura.
  const targetAccountId = calendars.find((c) => c.id === draft.calendarId)?.account_id;
  const caldavTarget = accounts.find((a) => a.id === targetAccountId)?.provider === "caldav";
  const moveBlocked = moving
    ? moveBlockReason(
        calendars.find((c) => c.id === currentCalendarId),
        calendars.find((c) => c.id === draft.calendarId),
        accounts,
        { attendees: draft.attendees.length, conferenceUrl: draft.conferenceUrl },
      )
    : null;

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (readOnly) return;
    const problems = validateDraft(draft);
    setErrors(problems);
    // Il motivo del blocco è già sotto il selettore del calendario.
    if (problems.length > 0 || moveBlocked) return;
    if (ref) return setChoosingScope(true);
    const fields = draftToFields(draft);
    try {
      if (event && detail.data && moving) {
        await move.mutateAsync({ eventId: detail.data.event.id, event: fields, attendees: draft.attendees, reminders: draft.reminders });
      } else if (event && detail.data) {
        await update.mutateAsync({ event: { ...detail.data.event, ...fields }, attendees: draft.attendees, reminders: draft.reminders });
      } else {
        await create.mutateAsync({ event: fields, attendees: caldavTarget ? [] : draft.attendees, reminders: draft.reminders });
      }
      close();
    } catch (err) {
      setErrors([errorText(err)]);
    }
  };

  const saveWithScope = async (scope: SeriesScope) => {
    if (!ref || !detail.data) return;
    try {
      await saveOccurrence.mutateAsync({
        ref,
        scope,
        fields: draftToFields(draft),
        attendees: draft.attendees,
        reminders: draft.reminders,
        exception: ref.isException ? detail.data.event : undefined,
      });
      close();
    } catch (err) {
      setChoosingScope(false);
      setErrors([errorText(err)]);
    }
  };

  const deleteWithScope = async (scope: SeriesScope) => {
    if (!ref) return;
    try {
      await removeScoped.mutateAsync({ ref, scope });
      close();
    } catch (err) {
      setConfirmDelete(false);
      setErrors([errorText(err)]);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-rail/45 p-4" onClick={onBackdropClick}>
      <form
        onSubmit={submit}
        className="max-h-full w-full max-w-lg space-y-3 overflow-y-auto rounded-lg border bg-popover p-5 text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]"
        style={{ borderTop: `5px solid ${draft.color ?? calendars.find((c) => c.id === draft.calendarId)?.color ?? "var(--border)"}` }}
      >
        {failedId && (
          <div role="alert" className="space-y-2 rounded-md border border-destructive/40 bg-destructive/5 p-3 text-sm">
            <p>
              {t("events.editor.rejected")}
              {detail.data?.sync_error && <span className="block text-xs text-muted-foreground">{detail.data.sync_error}</span>}
            </p>
            <div className="flex flex-wrap gap-2">
              <Button
                type="button"
                variant="outline"
                disabled={retryFailed.isPending}
                onClick={() => retryFailed.mutateAsync(failedId).then(close, (err) => setErrors([errorText(err)]))}
              >
                {t("events.editor.retry")}
              </Button>
              <Button
                type="button"
                variant="outline"
                disabled={discardFailed.isPending}
                onClick={() => discardFailed.mutateAsync(failedId).then(close, (err) => setErrors([errorText(err)]))}
              >
                {t("events.editor.discard")}
              </Button>
            </div>
          </div>
        )}
        <fieldset disabled={readOnly} className="min-w-0 space-y-3">
          <input
            autoFocus
            placeholder={t("events.editor.title")}
            value={draft.title}
            onChange={(e) => set("title", e.target.value)}
            className="w-full border-b bg-transparent pb-1.5 text-[22px] font-bold tracking-[-0.01em] outline-none placeholder:font-semibold placeholder:text-muted-foreground/70"
          />

          <div className="grid grid-cols-3 gap-2">
            <Field label={t("events.editor.date")}>
              <input type="date" required value={draft.date} onChange={(e) => set("date", e.target.value)} className={inputClass} />
            </Field>
            <Field label={t("events.editor.startTime")}>
              <input type="time" disabled={draft.allDay} value={draft.startTime} onChange={(e) => set("startTime", e.target.value)} className={inputClass} />
            </Field>
            <Field label={t("events.editor.endTime")}>
              <input type="time" disabled={draft.allDay} value={draft.endTime} onChange={(e) => set("endTime", e.target.value)} className={inputClass} />
            </Field>
          </div>

          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={draft.allDay} onChange={(e) => set("allDay", e.target.checked)} />
            {t("events.editor.allDay")}
          </label>

          <Field label={t("events.editor.calendar")}>
            {readOnly ? (
              <input readOnly value={sourceCalendar?.name ?? ""} className={inputClass} />
            ) : (
              <select required value={draft.calendarId} onChange={(e) => set("calendarId", e.target.value)} className={inputClass}>
                {writable.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </select>
            )}
            {moving && (
              <span className={`block text-xs ${moveBlocked ? "text-destructive" : "text-muted-foreground"}`}>
                {moveBlocked ??
                  (ref ? t("events.editor.moveSeries") : t("events.editor.moveEvent"))}
              </span>
            )}
          </Field>

          <Field label={t("events.editor.location")}>
            <input value={draft.location} onChange={(e) => set("location", e.target.value)} className={inputClass} />
          </Field>
          <Field label={t("events.editor.description")}>
            <textarea rows={3} value={draft.description} onChange={(e) => set("description", e.target.value)} className={inputClass} />
          </Field>

          <div className="grid grid-cols-2 gap-2">
            <Field label={t("events.editor.recurrence")}>
              <select
                value={draft.recurrence}
                onChange={(e) => {
                  const freq = e.target.value as RecurrenceFreq;
                  const days = freq === "weekly" && draft.recurrenceDays.length === 0 ? [weekdayOf(new Date(`${draft.date}T00:00`))] : draft.recurrenceDays;
                  setDraft({ ...draft, recurrence: freq, recurrenceDays: days });
                }}
                className={inputClass}
              >
                <option value="none">{t("events.recurrence.none")}</option>
                <option value="daily">{t("events.recurrence.daily")}</option>
                <option value="weekly">{t("events.recurrence.weekly")}</option>
                <option value="monthly">{t("events.recurrence.monthly")}</option>
                <option value="yearly">{t("events.recurrence.yearly")}</option>
                {draft.recurrence === "custom" && <option value="custom">{t("events.recurrence.custom")}</option>}
              </select>
            </Field>
            <Field label={t("events.editor.status")}>
              <select value={draft.status} onChange={(e) => set("status", e.target.value as EventDraft["status"])} className={inputClass}>
                <option value="busy">{t("events.editor.busy")}</option>
                <option value="free">{t("events.editor.free")}</option>
              </select>
            </Field>
          </div>

          {draft.recurrence === "weekly" && (
            <div className="flex flex-wrap gap-1" role="group" aria-label={t("events.editor.weekdays")}>
              {WEEKDAYS.map((d) => {
                const on = draft.recurrenceDays.includes(d);
                return (
                  <button
                    key={d}
                    type="button"
                    aria-pressed={on}
                    onClick={() => set("recurrenceDays", on ? draft.recurrenceDays.filter((x) => x !== d) : [...draft.recurrenceDays, d])}
                    className={`rounded-md border px-2.5 py-1 text-xs ${on ? "border-primary bg-primary text-primary-foreground" : "hover:bg-accent"}`}
                  >
                    {weekdayLabel(d)}
                  </button>
                );
              })}
            </div>
          )}
          {ref && !readOnly && (
            <p className="text-xs text-muted-foreground">
              {t("events.editor.occurrenceNote")}
            </p>
          )}
          {event && !ref && !readOnly && draft.recurrence !== "none" && <p className="text-xs text-muted-foreground">{t("events.editor.editingSeries")}</p>}

          <Field label={t("events.editor.conference")}>
            <div className="flex gap-2">
              <input
                type="url"
                placeholder="https://meet.google.com/..."
                value={draft.conferenceUrl}
                onChange={(e) => set("conferenceUrl", e.target.value)}
                className={inputClass}
              />
              {isValidConferenceUrl(draft.conferenceUrl) && (
                <Button type="button" variant="outline" className="shrink-0" onClick={() => void openExternal(draft.conferenceUrl)}>
                  {t("events.editor.joinMeeting")}
                </Button>
              )}
            </div>
          </Field>
          <AppearanceField
            value={{ color: draft.color, icon: draft.icon, pattern: draft.pattern }}
            calendarColor={calendars.find((c) => c.id === draft.calendarId)?.color ?? "#9AA1BD"}
            onChange={(v) => setDraft({ ...draft, ...v })}
          />
          <AttendeesField
            value={draft.attendees}
            onChange={(v) => set("attendees", v)}
            readOnlyNote={caldavTarget ? t("events.attendees.caldavNote") : undefined}
          />
          <RemindersField value={draft.reminders} onChange={(v) => set("reminders", v)} />
        </fieldset>

        {errors.length > 0 && (
          <ul role="alert" className="space-y-0.5 text-sm text-destructive">
            {errors.map((m) => (
              <li key={m}>{m}</li>
            ))}
          </ul>
        )}

        {choosingScope && (
          <ScopeChoice
            question={t("events.editor.applyTo")}
            note={
              moving
                ? t("events.editor.applyNoteMove")
                : t("events.editor.applyNote")
            }
            scopes={moving ? ["all"] : undefined}
            disabled={saveOccurrence.isPending}
            onChoose={(scope) => void saveWithScope(scope)}
            onCancel={() => setChoosingScope(false)}
          />
        )}
        {confirmDelete && ref && (
          <ScopeChoice
            question={t("events.editor.deleteQuestion")}
            destructive
            disabled={removeScoped.isPending}
            onChoose={(scope) => void deleteWithScope(scope)}
            onCancel={() => setConfirmDelete(false)}
          />
        )}

        {readOnly && (
          <div className="flex items-center justify-between gap-3 pt-1">
            <p className="text-xs text-muted-foreground">{t("events.editor.readOnly")}</p>
            <div className="flex shrink-0 gap-2">
              {isValidConferenceUrl(draft.conferenceUrl) && (
                <Button type="button" variant="outline" onClick={() => void openExternal(draft.conferenceUrl)}>
                  {t("events.editor.joinMeeting")}
                </Button>
              )}
              <Button type="button" onClick={close}>
                {t("events.editor.close")}
              </Button>
            </div>
          </div>
        )}
        {!readOnly && !choosingScope && !confirmDelete && (
          <div className="flex items-center justify-between pt-1">
            {event ? (
              <Button
                type="button"
                variant="destructive"
                onClick={async () => {
                  if (ref || draft.existingRule) return setConfirmDelete(true);
                  await remove.mutateAsync(event.id);
                  close();
                }}
              >
                {t("events.editor.delete")}
              </Button>
            ) : (
              <span />
            )}
            <div className="flex gap-2">
              <Button type="button" variant="ghost" onClick={close}>
                {t("events.editor.cancel")}
              </Button>
              <Button type="submit" disabled={create.isPending || update.isPending}>
                {t("events.editor.save")}
              </Button>
            </div>
          </div>
        )}
        {confirmDelete && !ref && event && (
          <div className="flex flex-wrap items-center gap-2">
            <Button
              type="button"
              variant="destructive"
              onClick={async () => {
                await remove.mutateAsync(event.id);
                close();
              }}
            >
              {t("events.editor.deleteSeries")}
            </Button>
            <Button type="button" variant="ghost" onClick={() => setConfirmDelete(false)}>
              {t("events.editor.back")}
            </Button>
          </div>
        )}
      </form>
    </div>
  );
}
