import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { openExternal } from "@/providers/externalLinks";
import { useCalendars, useCreateEvent, useDeleteEvent, useEventDetail, useUpdateEvent } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { isValidConferenceUrl } from "@/utils/validation";
import { AttendeesField } from "./AttendeesField";
import { draftToFields, emptyDraft, eventToDraft, validateDraft, type EventDraft, type RecurrencePreset } from "./draft";
import { RemindersField } from "./RemindersField";

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-xs text-muted-foreground">{label}</span>
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
  const { open, event, slot, close } = useEditorStore();
  const { data: calendars = [] } = useCalendars();
  const create = useCreateEvent();
  const update = useUpdateEvent();
  const remove = useDeleteEvent();
  const detail = useEventDetail(open && event ? event.id : null);
  const [draft, setDraft] = useState<EventDraft | null>(null);
  const [errors, setErrors] = useState<string[]>([]);

  const writable = calendars.filter((c) => !c.read_only);

  // Il draft si inizializza all'apertura (e quando arriva il dettaglio di un evento esistente);
  // un refetch dello stesso evento non deve azzerare ciò che l'utente sta scrivendo.
  const detailId = detail.data?.event.id;
  useEffect(() => {
    setErrors([]);
    if (!open) return setDraft(null);
    if (!event) return setDraft(emptyDraft((writable.find((c) => c.visible) ?? writable[0])?.id ?? "", slot));
    if (detail.data) setDraft(eventToDraft(detail.data.event, detail.data.attendees, detail.data.reminders));
  }, [open, event?.id, detailId, slot]);

  if (open && event && !draft) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" onMouseDown={close}>
        <div className="rounded-xl border bg-popover p-5 text-sm shadow-xl" onMouseDown={(e) => e.stopPropagation()}>
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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" onMouseDown={close}>
      <form
        onSubmit={submit}
        onMouseDown={(e) => e.stopPropagation()}
        className="max-h-full w-full max-w-lg space-y-3 overflow-y-auto rounded-xl border bg-popover p-5 text-popover-foreground shadow-xl"
      >
        <input
          autoFocus
          placeholder="Titolo"
          value={draft.title}
          onChange={(e) => set("title", e.target.value)}
          className="w-full border-b bg-transparent pb-1 text-lg outline-none"
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
            <select value={draft.recurrence} onChange={(e) => set("recurrence", e.target.value as RecurrencePreset)} className={inputClass}>
              <option value="none">Non si ripete</option>
              <option value="daily">Ogni giorno</option>
              <option value="weekly">Ogni settimana</option>
              <option value="monthly">Ogni mese</option>
              <option value="yearly">Ogni anno</option>
            </select>
          </Field>
          <Field label="Stato">
            <select value={draft.status} onChange={(e) => set("status", e.target.value as EventDraft["status"])} className={inputClass}>
              <option value="busy">Occupato</option>
              <option value="free">Libero</option>
            </select>
          </Field>
        </div>

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

        <div className="flex items-center justify-between pt-1">
          {event ? (
            <Button
              type="button"
              variant="destructive"
              onClick={async () => {
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
      </form>
    </div>
  );
}
