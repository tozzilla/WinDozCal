import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { openExternal } from "@/providers/externalLinks";
import {
  useCalendars,
  useCreateEvent,
  useDeleteEvent,
  useDeleteOccurrenceScoped,
  useEventDetail,
  useSaveOccurrence,
  useUpdateEvent,
} from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { occurrenceRef } from "@/utils/events";
import { isValidConferenceUrl } from "@/utils/validation";
import { AttendeesField } from "./AttendeesField";
import { draftToFields, emptyDraft, eventToDraft, toDateInput, toTimeInput, validateDraft, type EventDraft } from "./draft";
import { weekdayOf, WEEKDAYS, type RecurrenceFreq, type Weekday } from "./recurrence";
import { RemindersField } from "./RemindersField";
import { ScopeChoice } from "./ScopeChoice";
import type { SeriesScope } from "./seriesEdit";

const WEEKDAY_LABEL: Record<Weekday, string> = { MO: "Lun", TU: "Mar", WE: "Mer", TH: "Gio", FR: "Ven", SA: "Sab", SU: "Dom" };

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-[11px] font-bold tracking-[0.1em] text-muted-foreground uppercase">{label}</span>
      {children}
    </label>
  );
}

function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return "Errore sconosciuto.";
}

/** Editor evento minimo (PRD §7). Esc lo chiude (gestito da useKeyboardShortcuts). */
export function EventEditor() {
  const { open, openedAt, event, slot, preset, close } = useEditorStore();
  const { data: calendars = [] } = useCalendars();
  const create = useCreateEvent();
  const update = useUpdateEvent();
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
      const base = emptyDraft((writable.find((c) => c.visible) ?? writable[0])?.id ?? "", slot);
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
              <p className="mb-3 text-destructive">Impossibile caricare l&apos;evento: {errorMessage(detail.error)}</p>
              <Button variant="outline" onClick={close}>
                Chiudi
              </Button>
            </>
          ) : (
            "Caricamento dell'evento…"
          )}
        </div>
      </div>
    );
  }

  if (!open || !draft) return null;

  const set = <K extends keyof EventDraft>(key: K, value: EventDraft[K]) => setDraft({ ...draft, [key]: value });

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const problems = validateDraft(draft);
    setErrors(problems);
    if (problems.length > 0) return;
    if (ref) return setChoosingScope(true);
    const fields = draftToFields(draft);
    try {
      if (event && detail.data) {
        await update.mutateAsync({ event: { ...detail.data.event, ...fields }, attendees: draft.attendees, reminders: draft.reminders });
      } else {
        await create.mutateAsync({ event: fields, attendees: draft.attendees, reminders: draft.reminders });
      }
      close();
    } catch (err) {
      setErrors([errorMessage(err)]);
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
      setErrors([errorMessage(err)]);
    }
  };

  const deleteWithScope = async (scope: SeriesScope) => {
    if (!ref) return;
    try {
      await removeScoped.mutateAsync({ ref, scope });
      close();
    } catch (err) {
      setConfirmDelete(false);
      setErrors([errorMessage(err)]);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-rail/45 p-4" onClick={onBackdropClick}>
      <form
        onSubmit={submit}
        className="max-h-full w-full max-w-lg space-y-3 overflow-y-auto rounded-lg border bg-popover p-5 text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]"
        style={{ borderTop: `5px solid ${calendars.find((c) => c.id === draft.calendarId)?.color ?? "var(--border)"}` }}
      >
        <input
          autoFocus
          placeholder="Titolo"
          value={draft.title}
          onChange={(e) => set("title", e.target.value)}
          className="w-full border-b bg-transparent pb-1.5 text-[22px] font-bold tracking-[-0.01em] outline-none placeholder:font-semibold placeholder:text-muted-foreground/70"
        />

        <div className="grid grid-cols-3 gap-2">
          <Field label="Data">
            <input type="date" required value={draft.date} onChange={(e) => set("date", e.target.value)} className={inputClass} />
          </Field>
          <Field label="Ora inizio">
            <input type="time" disabled={draft.allDay} value={draft.startTime} onChange={(e) => set("startTime", e.target.value)} className={inputClass} />
          </Field>
          <Field label="Ora fine">
            <input type="time" disabled={draft.allDay} value={draft.endTime} onChange={(e) => set("endTime", e.target.value)} className={inputClass} />
          </Field>
        </div>

        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={draft.allDay} onChange={(e) => set("allDay", e.target.checked)} />
          Tutto il giorno
        </label>

        <Field label="Calendario">
          <select required value={draft.calendarId} onChange={(e) => set("calendarId", e.target.value)} className={inputClass}>
            {writable.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Luogo">
          <input value={draft.location} onChange={(e) => set("location", e.target.value)} className={inputClass} />
        </Field>
        <Field label="Descrizione / note">
          <textarea rows={3} value={draft.description} onChange={(e) => set("description", e.target.value)} className={inputClass} />
        </Field>

        <div className="grid grid-cols-2 gap-2">
          <Field label="Ricorrenza">
            <select
              value={draft.recurrence}
              onChange={(e) => {
                const freq = e.target.value as RecurrenceFreq;
                const days = freq === "weekly" && draft.recurrenceDays.length === 0 ? [weekdayOf(new Date(`${draft.date}T00:00`))] : draft.recurrenceDays;
                setDraft({ ...draft, recurrence: freq, recurrenceDays: days });
              }}
              className={inputClass}
            >
              <option value="none">Non si ripete</option>
              <option value="daily">Ogni giorno</option>
              <option value="weekly">Ogni settimana</option>
              <option value="monthly">Ogni mese</option>
              <option value="yearly">Ogni anno</option>
              {draft.recurrence === "custom" && <option value="custom">Personalizzata (non modificabile qui)</option>}
            </select>
          </Field>
          <Field label="Stato">
            <select value={draft.status} onChange={(e) => set("status", e.target.value as EventDraft["status"])} className={inputClass}>
              <option value="busy">Occupato</option>
              <option value="free">Libero</option>
            </select>
          </Field>
        </div>

        {draft.recurrence === "weekly" && (
          <div className="flex flex-wrap gap-1" role="group" aria-label="Giorni della settimana">
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
                  {WEEKDAY_LABEL[d]}
                </button>
              );
            })}
          </div>
        )}
        {ref && (
          <p className="text-xs text-muted-foreground">
            Occorrenza di una serie: al salvataggio scegli se modificare solo questo evento, anche i successivi o tutta la serie.
          </p>
        )}
        {event && !ref && draft.recurrence !== "none" && <p className="text-xs text-muted-foreground">Stai modificando l&apos;intera serie.</p>}

        <Field label="Videoconferenza (link)">
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
                Join meeting
              </Button>
            )}
          </div>
        </Field>
        <AttendeesField value={draft.attendees} onChange={(v) => set("attendees", v)} />
        <RemindersField value={draft.reminders} onChange={(v) => set("reminders", v)} />

        {errors.length > 0 && (
          <ul role="alert" className="space-y-0.5 text-sm text-destructive">
            {errors.map((m) => (
              <li key={m}>{m}</li>
            ))}
          </ul>
        )}

        {choosingScope && (
          <ScopeChoice
            question="Applicare la modifica a"
            note="Se cambi data, orari o ricorrenza di tutta la serie o dei successivi, le modifiche già fatte alle singole occorrenze interessate vanno perse."
            disabled={saveOccurrence.isPending}
            onChoose={(scope) => void saveWithScope(scope)}
            onCancel={() => setChoosingScope(false)}
          />
        )}
        {confirmDelete && ref && (
          <ScopeChoice
            question="Eliminare"
            destructive
            disabled={removeScoped.isPending}
            onChoose={(scope) => void deleteWithScope(scope)}
            onCancel={() => setConfirmDelete(false)}
          />
        )}

        {!choosingScope && !confirmDelete && (
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
                Elimina
              </Button>
            ) : (
              <span />
            )}
            <div className="flex gap-2">
              <Button type="button" variant="ghost" onClick={close}>
                Annulla
              </Button>
              <Button type="submit" disabled={create.isPending || update.isPending}>
                Salva
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
              Elimina tutta la serie
            </Button>
            <Button type="button" variant="ghost" onClick={() => setConfirmDelete(false)}>
              Indietro
            </Button>
          </div>
        )}
      </form>
    </div>
  );
}
