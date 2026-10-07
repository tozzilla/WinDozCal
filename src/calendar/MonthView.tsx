import { cn } from "@/lib/utils";
import { formatWeekday, getMonthGrid, getWeekDays, isSameDay } from "@/utils/date";
import { eventKey, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import { eventSurface } from "./palinsesto";
import type { CalendarRendererProps } from "./types";

const MAX_CHIPS = 3;

/** Vista mensile compatta (PRD §5): al massimo MAX_CHIPS eventi per cella, poi "+N altri" porta al giorno. */
export function MonthView({ date, events, calendars, onSelectSlot, onSelectEvent, onShowDay }: CalendarRendererProps) {
  const today = new Date();
  const grid = getMonthGrid(date);

  return (
    <div className="flex h-full flex-col bg-grid">
      <div className="grid grid-cols-7 border-b bg-card">
        {getWeekDays(date).map((d) => (
          <div key={d.getDay()} className="px-2.5 py-2 text-[12px] font-semibold tracking-[0.08em] text-muted-foreground uppercase">
            {formatWeekday(d)}
          </div>
        ))}
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-7 grid-rows-6">
        {grid.map((day) => {
          const dayEvents = eventsOnDay(events, day);
          const hidden = dayEvents.length - MAX_CHIPS;
          return (
            <div
              key={day.toISOString()}
              className={cn("min-h-0 overflow-hidden border-b border-l border-grid-line p-1.5", isSameDay(day, today) && "bg-today")}
              onDoubleClick={(e) => {
                if ((e.target as HTMLElement).closest("button")) return;
                const start = new Date(day.getFullYear(), day.getMonth(), day.getDate(), 9);
                onSelectSlot(start, new Date(start.getTime() + 60 * 60_000));
              }}
            >
              <div
                className={cn(
                  "mb-1 px-0.5 text-[15px] leading-none font-bold",
                  day.getMonth() !== date.getMonth() && "font-semibold text-muted-foreground/70",
                  isSameDay(day, today) && "text-onair",
                )}
              >
                {day.getDate()}
              </div>
              {dayEvents.slice(0, MAX_CHIPS).map((e) => (
                <button
                  key={eventKey(e)}
                  type="button"
                  onClick={() => onSelectEvent(e)}
                  className="mb-0.5 flex w-full items-center truncate rounded-[3px] px-1.5 py-px text-left text-[12px] font-semibold outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  style={{ ...eventSurface(calendarColor(calendars, e.calendar_id), e.status === "free"), borderLeftWidth: 3 }}
                >
                  <span className="truncate">{e.title}</span>
                </button>
              ))}
              {hidden > 0 && (
                <button type="button" onClick={() => onShowDay(day)} className="px-1 text-[12px] font-semibold text-muted-foreground hover:text-foreground">
                  +{hidden} altri
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
