import { renderers } from "@/calendar/renderers";
import { useCalendars, useEvents } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { toIsoWithOffset } from "@/utils/date";

/** Collega store, Calendar Service e renderer: conosce solo l'interfaccia `CalendarRenderer`. */
export function CalendarSurface() {
  const { view, currentDate } = useUiStore();
  const { openNew, openExisting } = useEditorStore();
  const renderer = renderers[view];
  const range = renderer.getRange(currentDate);
  const { data: events = [] } = useEvents(toIsoWithOffset(range.start), toIsoWithOffset(range.end));
  const { data: calendars = [] } = useCalendars();

  return (
    <div className="min-h-0 flex-1">
      <renderer.Component
        date={currentDate}
        events={events}
        calendars={calendars}
        onSelectSlot={(start, end) => openNew({ start, end })}
        onSelectEvent={openExisting}
      />
    </div>
  );
}
