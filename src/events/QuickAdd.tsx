import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { calendarColor } from "@/calendar/colors";
import { useCalendars, useCreateEvent } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { formatTime, localTimezone, toIsoWithOffset } from "@/utils/date";
import { parseQuickAdd } from "@/utils/quickAdd";

const dayFormat = new Intl.DateTimeFormat("it-IT", { weekday: "long", day: "numeric", month: "long", year: "numeric" });

/** Quick Add `Ctrl + N` (PRD §8, ADR 012): testo libero it/en, anteprima, Invio per creare. */
export function QuickAdd() {
  const { quickAddOpen, setQuickAddOpen, setCurrentDate, setNotice } = useUiStore();
  const openNew = useEditorStore((s) => s.openNew);
  const { data: calendars = [] } = useCalendars();
  const create = useCreateEvent();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!quickAddOpen) {
      setText("");
      setError(null);
    }
  }, [quickAddOpen]);

  const parsed = useMemo(() => (text.trim() ? parseQuickAdd(text) : null), [text]);
  const writable = calendars.filter((c) => !c.read_only);
  const calendar = writable.find((c) => c.visible) ?? writable[0];

  if (!quickAddOpen) return null;

  const close = () => setQuickAddOpen(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!parsed || !calendar) return;
    if (!parsed.title) return setError("Scrivi anche il titolo dell'evento.");
    try {
      await create.mutateAsync({
        event: {
          calendar_id: calendar.id,
          title: parsed.title,
          description: null,
          location: null,
          conference_url: null,
          start: toIsoWithOffset(parsed.start),
          end: toIsoWithOffset(parsed.end),
          timezone: localTimezone(),
          all_day: parsed.allDay,
          recurrence_rule: null,
          status: "busy",
          color: null,
          icon: null,
          pattern: null,
        },
        attendees: [],
        reminders: [],
      });
      setCurrentDate(parsed.start);
      setNotice(`Creato "${parsed.title}" il ${dayFormat.format(parsed.start)}.`);
      close();
    } catch (err) {
      setError(err instanceof Error ? err.message : String((err as { message?: unknown })?.message ?? err));
    }
  };

  const moreOptions = () => {
    close();
    openNew(parsed ? { start: parsed.start, end: parsed.end } : undefined, { title: parsed?.title ?? text.trim(), allDay: parsed?.allDay ?? false });
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-rail/35 p-4 pt-[18vh]" onClick={(e) => e.target === e.currentTarget && close()}>
      <form role="dialog" aria-label="Quick Add" onSubmit={submit} className="h-fit w-full max-w-lg space-y-3 rounded-lg border bg-popover p-4 text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]">
        <input
          autoFocus
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setError(null);
          }}
          placeholder="Es. Riunione con il team domani alle 15"
          className="w-full border-b bg-transparent pb-2 text-[20px] font-semibold outline-none placeholder:font-normal placeholder:text-muted-foreground/70"
        />
        <div aria-live="polite" className="min-h-10 text-sm">
          {parsed && calendar ? (
            <div className="flex items-start gap-2">
              <span className="mt-0.5 h-9 w-1.5 shrink-0 rounded-[1px]" style={{ background: calendarColor(calendars, calendar.id) }} />
              <div>
                <p className="text-[15px] font-bold">{parsed.title || "(senza titolo)"}</p>
                <p className="text-muted-foreground">
                  {dayFormat.format(parsed.start)}
                  {parsed.allDay ? " · tutto il giorno" : ` · ${formatTime(parsed.start)}–${formatTime(parsed.end)}`} · {calendar.name}
                </p>
              </div>
            </div>
          ) : (
            <p className="text-muted-foreground">
              {!calendar ? "Nessun calendario scrivibile." : text.trim() ? "Nessuna data riconosciuta: aggiungi giorno o ora, oppure usa Altre opzioni." : "Scrivi titolo, giorno e ora, in italiano o in inglese."}
            </p>
          )}
          {error && <p role="alert" className="mt-1 text-destructive">{error}</p>}
        </div>
        <div className="flex justify-between">
          <Button type="button" variant="ghost" onClick={moreOptions}>
            Altre opzioni
          </Button>
          <Button type="submit" disabled={!parsed || !calendar || create.isPending}>
            Crea
          </Button>
        </div>
      </form>
    </div>
  );
}
