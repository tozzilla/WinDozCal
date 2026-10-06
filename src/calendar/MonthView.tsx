import { cn } from "@/lib/utils";
import { formatWeekday, getMonthGrid, getWeekDays, isSameDay } from "@/utils/date";
import { eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import type { CalendarRendererProps } from "./types";

const MAX_CHIPS = 3;

/** Vista mensile con eventi compatti (PRD §5). */
export function MonthView({ date, events, calendars, onSelectSlot, onSelectEvent }: CalendarRendererProps) {
  const today = new Date();
  const grid = getMonthGrid(date);

  return (
    <div className="flex h-full flex-col">
      <div className="grid grid-cols-7 border-b">
        {getWeekDays(date).map((d) => (
          <div key={d.getDay()} className="px-2 py-1.5 text-xs text-muted-foreground">
            {formatWeekday(d)}
          </div>
        ))}
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-7 grid-rows-6">
        {grid.map((day) => {
          const dayEvents = eventsOnDay(events, day);
          return (
            <div
              key={day.toISOString()}
              className="min-h-0 overflow-hidden border-b border-l p-1"
              onDoubleClick={() => {
                const start = new Date(day.getFullYear(), day.getMonth(), day.getDate(), 9);
                onSelectSlot(start, new Date(start.getTime() + 60 * 60_000));
              }}
            >
              <div
                className={cn(
                  "mb-0.5 flex size-6 items-center justify-center rounded-full text-xs",
                  day.getMonth() !== date.getMonth() && "text-muted-foreground",
                  isSameDay(day, today) && "bg-primary text-primary-foreground",
                )}
              >
                {day.getDate()}
              </div>
              {dayEvents.slice(0, MAX_CHIPS).map((e) => (
                <button
                  key={e.id}
                  type="button"
                  onClick={() => onSelectEvent(e)}
                  onDoubleClick={(ev) => ev.stopPropagation()}
                  className="mb-0.5 flex w-full items-center gap-1 truncate text-left text-xs"
                >
                  <span className="size-2 shrink-0 rounded-full" style={{ backgroundColor: calendarColor(calendars, e.calendar_id) }} />
                  <span className="truncate">{e.title}</span>
                </button>
              ))}
              {dayEvents.length > MAX_CHIPS && (
                <div className="text-xs text-muted-foreground">+{dayEvents.length - MAX_CHIPS}</div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
