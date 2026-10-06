import { addDays, formatTime, startOfDay } from "@/utils/date";
import { eventInterval, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import type { CalendarRendererProps } from "./types";

export const AGENDA_DAYS = 30;

/** Lista cronologica degli eventi futuri, raggruppati per giorno (PRD §5). */
export function AgendaView({ date, events, calendars, onSelectEvent }: CalendarRendererProps) {
  const first = startOfDay(date);
  const days = Array.from({ length: AGENDA_DAYS }, (_, i) => addDays(first, i))
    .map((day) => ({ day, items: eventsOnDay(events, day) }))
    .filter((g) => g.items.length > 0);

  if (days.length === 0) {
    return <div className="p-6 text-sm text-muted-foreground">Nessun evento nei prossimi {AGENDA_DAYS} giorni.</div>;
  }

  return (
    <div className="h-full overflow-y-auto px-6 py-4">
      {days.map(({ day, items }) => (
        <section key={day.toISOString()} className="mb-5">
          <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {new Intl.DateTimeFormat("it-IT", { weekday: "long", day: "numeric", month: "long" }).format(day)}
          </h3>
          {items.map((e) => (
            <button
              key={e.id}
              type="button"
              onClick={() => onSelectEvent(e)}
              className="flex w-full items-center gap-3 rounded px-2 py-1.5 text-left text-sm hover:bg-accent"
            >
              <span className="size-2.5 shrink-0 rounded-full" style={{ backgroundColor: calendarColor(calendars, e.calendar_id) }} />
              <span className="w-12 shrink-0 tabular-nums text-muted-foreground">
                {e.all_day ? "Tutto il giorno" : formatTime(eventInterval(e).start)}
              </span>
              <span className="truncate">{e.title}</span>
            </button>
          ))}
        </section>
      ))}
    </div>
  );
}
