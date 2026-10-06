import { addDays, formatTime, isSameDay, startOfDay } from "@/utils/date";
import { eventInterval, eventKey, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import type { CalendarRendererProps } from "./types";

export const AGENDA_DAYS = 30;

const dateFormat = new Intl.DateTimeFormat("it-IT", { weekday: "long", day: "numeric", month: "long" });

/** OGGI / DOMANI / data estesa (PRD §5); il maiuscolo è reso via CSS. */
function dayLabel(day: Date, today: Date): string {
  if (isSameDay(day, today)) return "Oggi";
  if (isSameDay(day, addDays(today, 1))) return "Domani";
  return dateFormat.format(day);
}

/** Lista cronologica degli eventi futuri, raggruppati per giorno (PRD §5). Oggi mostra solo ciò che non è finito. */
export function AgendaView({ date, events, calendars, onSelectEvent }: CalendarRendererProps) {
  const now = new Date();
  const first = startOfDay(date);
  const groups = Array.from({ length: AGENDA_DAYS }, (_, i) => addDays(first, i))
    .map((day) => ({
      day,
      items: eventsOnDay(events, day).filter((e) => !isSameDay(day, now) || e.all_day || eventInterval(e).end > now),
    }))
    .filter((g) => g.items.length > 0);

  if (groups.length === 0) {
    return <div className="p-6 text-sm text-muted-foreground">Nessun evento nei prossimi {AGENDA_DAYS} giorni.</div>;
  }

  return (
    <div className="h-full overflow-y-auto px-6 py-4">
      {groups.map(({ day, items }) => (
        <section key={day.toISOString()} className="mb-5">
          <h3 className="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{dayLabel(day, now)}</h3>
          {items.map((e) => (
            <button
              key={eventKey(e)}
              type="button"
              onClick={() => onSelectEvent(e)}
              className="flex w-full items-center gap-3 rounded px-2 py-1.5 text-left text-sm hover:bg-accent"
            >
              <span className="size-2.5 shrink-0 rounded-full" style={{ backgroundColor: calendarColor(calendars, e.calendar_id) }} />
              <span className="w-24 shrink-0 tabular-nums text-muted-foreground">
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
