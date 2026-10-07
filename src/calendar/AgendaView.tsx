import { addDays, formatTime, isSameDay, startOfDay } from "@/utils/date";
import { eventInterval, eventKey, eventsOnDay } from "@/utils/events";
import { cn } from "@/lib/utils";
import { calendarColor } from "./colors";
import { EventIcon } from "./appearance";
import { broadcastMarks, MARK_LABEL } from "./palinsesto";
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
    return <div className="h-full bg-grid p-8 text-[15px] text-muted-foreground">Nessun evento nei prossimi {AGENDA_DAYS} giorni.</div>;
  }

  const marks = broadcastMarks(events, calendars, now);

  return (
    <div className="h-full overflow-y-auto bg-grid px-8 py-5">
      {groups.map(({ day, items }) => (
        <section key={day.toISOString()} className="mb-6 max-w-3xl">
          <h3 className={cn("mb-1 border-b pb-1.5 text-[12px] font-bold tracking-[0.14em] uppercase", isSameDay(day, now) ? "text-onair" : "text-muted-foreground")}>
            {dayLabel(day, now)}
          </h3>
          {items.map((e) => (
            <button
              key={eventKey(e)}
              type="button"
              onClick={() => onSelectEvent(e)}
              className="flex w-full items-center gap-4 border-b border-grid-line px-1 py-2.5 text-left outline-none hover:bg-secondary/60 focus-visible:ring-2 focus-visible:ring-ring"
            >
              <span className={cn("w-28 shrink-0 font-bold", e.all_day ? "text-[13px] text-muted-foreground" : "text-[17px]")}>
                {e.all_day ? "Tutto il giorno" : formatTime(eventInterval(e).start)}
              </span>
              <span aria-hidden className="h-5 w-1.5 shrink-0 rounded-[1px]" style={{ backgroundColor: e.color ?? calendarColor(calendars, e.calendar_id) }} />
              <EventIcon name={e.icon} className="size-4 shrink-0 text-muted-foreground" />
              <span className="truncate text-[15px] font-semibold">{e.title}</span>
              {marks.get(eventKey(e)) && (
                <span
                  className={cn(
                    "ml-auto shrink-0 rounded-[3px] px-1.5 py-px text-[10px] font-bold tracking-[0.08em]",
                    marks.get(eventKey(e)) === "onair" ? "bg-onair text-onair-foreground" : marks.get(eventKey(e)) === "next" ? "bg-primary text-primary-foreground" : "bg-onair/12 text-onair",
                  )}
                >
                  {MARK_LABEL[marks.get(eventKey(e))!]}
                </span>
              )}
            </button>
          ))}
        </section>
      ))}
    </div>
  );
}
