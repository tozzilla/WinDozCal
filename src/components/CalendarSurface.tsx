import { renderers } from "@/calendar/renderers";
import { useCalendars, useChangeEventTime, useEvents } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { toIsoWithOffset } from "@/utils/date";

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

  return (
    <div className="min-h-0 flex-1">
      <renderer.Component
        date={currentDate}
        events={events}
        calendars={calendars}
        onSelectSlot={(start, end) => openNew({ start, end })}
        onSelectEvent={openExisting}
        onChangeEventTime={(event, start, end) =>
          changeTime.mutate(
            { event, start: toIsoWithOffset(start), end: toIsoWithOffset(end) },
            { onError: (err) => setNotice(`Spostamento non riuscito: ${errorText(err)}`) },
          )
        }
        onNotice={setNotice}
        onShowDay={(day) => {
          setCurrentDate(day);
          setView("day");
        }}
      />
    </div>
  );
}
