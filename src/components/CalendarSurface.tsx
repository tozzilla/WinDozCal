import { useState } from "react";
import { renderers } from "@/calendar/renderers";
import { ScopeChoice } from "@/events/ScopeChoice";
import { toNewEvent, type SeriesScope } from "@/events/seriesEdit";
import { fetchEventDetail, useCalendars, useChangeEventTime, useEvents, useSaveOccurrence } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import type { Event } from "@/types";
import { formatTime, toIsoWithOffset } from "@/utils/date";
import { occurrenceRef } from "@/utils/events";

function errorText(err: unknown): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return String(err);
}

/** Collega store, Calendar Service e renderer: conosce solo l'interfaccia `CalendarRenderer`. */
export function CalendarSurface() {
  const { view, currentDate, setView, setCurrentDate, setNotice } = useUiStore();
  const { openNew, openExisting } = useEditorStore();
  const renderer = renderers[view];
  const range = renderer.getRange(currentDate);
  const { data: events = [] } = useEvents(toIsoWithOffset(range.start), toIsoWithOffset(range.end));
  const { data: calendars = [] } = useCalendars();
  const changeTime = useChangeEventTime();
  const saveOccurrence = useSaveOccurrence();
  /** Occorrenza di serie trascinata, in attesa della scelta della portata (PRD §10). */
  const [pendingMove, setPendingMove] = useState<{ event: Event; start: Date; end: Date } | null>(null);

  const moveOccurrence = async (scope: SeriesScope) => {
    if (!pendingMove) return;
    const { event, start, end } = pendingMove;
    const ref = occurrenceRef(event);
    setPendingMove(null);
    if (!ref) return;
    try {
      const series = await fetchEventDetail(ref.seriesId);
      await saveOccurrence.mutateAsync({
        ref,
        scope,
        fields: { ...toNewEvent(series.event), start: toIsoWithOffset(start), end: toIsoWithOffset(end) },
        attendees: series.attendees.map((a) => ({ email: a.email, name: a.name })),
        reminders: series.reminders.map((r) => ({ minutes_before: r.minutes_before, type: r.type })),
      });
    } catch (err) {
      setNotice(`Spostamento non riuscito: ${errorText(err)}`);
    }
  };

  return (
    <div className="min-h-0 flex-1">
      <renderer.Component
        date={currentDate}
        events={events}
        calendars={calendars}
        onSelectSlot={(start, end) => openNew({ start, end })}
        onSelectEvent={openExisting}
        onChangeEventTime={(event, start, end) => {
          // Le eccezioni sono eventi a sé: si spostano direttamente. Le occorrenze espanse chiedono la portata.
          const ref = occurrenceRef(event);
          if (ref && !ref.isException) return setPendingMove({ event, start, end });
          changeTime.mutate(
            { event, start: toIsoWithOffset(start), end: toIsoWithOffset(end) },
            { onError: (err) => setNotice(`Spostamento non riuscito: ${errorText(err)}`) },
          );
        }}
        onNotice={setNotice}
        onShowDay={(day) => {
          setCurrentDate(day);
          setView("day");
        }}
      />
      {pendingMove && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-rail/45 p-4" onClick={(e) => e.target === e.currentTarget && setPendingMove(null)}>
          <div className="w-full max-w-lg rounded-lg border bg-popover p-5 text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]">
            <ScopeChoice
              question={`Spostare "${pendingMove.event.title}" alle ${formatTime(pendingMove.start)}–${formatTime(pendingMove.end)}: applicare a`}
              note="Spostando tutta la serie o i successivi, le modifiche già fatte alle singole occorrenze interessate vanno perse."
              onChoose={(scope) => void moveOccurrence(scope)}
              onCancel={() => setPendingMove(null)}
            />
          </div>
        </div>
      )}
    </div>
  );
}
