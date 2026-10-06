import { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useCalendars, useCreateEvent, useDeleteEvent, useUpdateEvent } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { draftToFields, emptyDraft, eventToDraft, type EventDraft, type RecurrencePreset } from "./draft";

const inputClass = "w-full rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50";

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block space-y-1">
      <span className="text-xs text-muted-foreground">{label}</span>
      {children}
    </label>
  );
}

/** Editor evento minimo (PRD §7). Esc lo chiude (gestito da useKeyboardShortcuts). */
export function EventEditor() {
  const { open, event, slot, close } = useEditorStore();
  const { data: calendars = [] } = useCalendars();
  const create = useCreateEvent();
  const update = useUpdateEvent();
  const remove = useDeleteEvent();
  const [draft, setDraft] = useState<EventDraft | null>(null);

  const writable = calendars.filter((c) => !c.read_only);

  useEffect(() => {
    if (!open) return setDraft(null);
    setDraft(event ? eventToDraft(event) : emptyDraft((writable.find((c) => c.visible) ?? writable[0])?.id ?? "", slot));
    // Il draft si inizializza all'apertura: i calendari che arrivano dopo non devono azzerarlo.
  }, [open, event, slot]);

  if (!open || !draft) return null;

  const set = <K extends keyof EventDraft>(key: K, value: EventDraft[K]) => setDraft({ ...draft, [key]: value });

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const fields = draftToFields(draft);
    if (event) await update.mutateAsync({ ...event, ...fields });
    else await create.mutateAsync(fields);
    close();
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
          required
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

        <fieldset className="space-y-2 rounded-md border border-dashed p-2">
          <legend className="px-1 text-xs text-muted-foreground">Non ancora salvati (campi non presenti nel contratto)</legend>
          <label className="flex items-center gap-2 text-sm">
            <input type="checkbox" checked={draft.videoconference} onChange={(e) => set("videoconference", e.target.checked)} />
            Videoconferenza
          </label>
          <Field label="Partecipanti (email separate da virgola)">
            <input value={draft.attendees} onChange={(e) => set("attendees", e.target.value)} className={inputClass} />
          </Field>
          <Field label="Promemoria (minuti prima)">
            <input
              type="number"
              min={0}
              value={draft.reminderMinutes ?? ""}
              onChange={(e) => set("reminderMinutes", e.target.value === "" ? null : Number(e.target.value))}
              className={inputClass}
            />
          </Field>
        </fieldset>

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
            <Button type="submit" disabled={!draft.calendarId}>
              Salva
            </Button>
          </div>
        </div>
      </form>
    </div>
  );
}
