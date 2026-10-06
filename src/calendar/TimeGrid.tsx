import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import type { Event } from "@/types";
import { addDays, formatTime, formatWeekday, isSameDay, startOfDay } from "@/utils/date";
import { eventInterval, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import type { CalendarRendererProps } from "./types";

const HOUR_HEIGHT = 48;
const HOURS = Array.from({ length: 24 }, (_, i) => i);

interface TimeGridProps extends CalendarRendererProps {
  days: Date[];
}

/** Griglia oraria condivisa da Giorno e Settimana. Gli eventi sovrapposti non sono ancora affiancati. */
export function TimeGrid({ days, events, calendars, onSelectSlot, onSelectEvent }: TimeGridProps) {
  const today = new Date();
  const scroller = useRef<HTMLDivElement>(null);

  // All'apertura la griglia parte dalle 7:00 invece che da mezzanotte.
  useEffect(() => {
    if (scroller.current) scroller.current.scrollTop = 7 * HOUR_HEIGHT;
  }, [days.length]);
  const cols = { gridTemplateColumns: `3.5rem repeat(${days.length}, minmax(0, 1fr))` };

  const slotFromClick = (day: Date, e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const hour = Math.max(0, Math.min(23, Math.floor((e.clientY - rect.top) / HOUR_HEIGHT)));
    const start = new Date(day.getFullYear(), day.getMonth(), day.getDate(), hour);
    onSelectSlot(start, new Date(start.getTime() + 60 * 60_000));
  };

  const chip = (e: Event) => (
    <button
      key={e.id}
      type="button"
      onClick={(ev) => {
        ev.stopPropagation();
        onSelectEvent(e);
      }}
      className="block w-full truncate rounded px-1.5 py-0.5 text-left text-xs font-medium text-white"
      style={{ backgroundColor: calendarColor(calendars, e.calendar_id) }}
    >
      {e.title}
    </button>
  );

  return (
    <div className="flex h-full flex-col">
      <div className="grid border-b" style={cols}>
        <div />
        {days.map((day) => (
          <div key={day.toISOString()} className="border-l px-2 py-1.5 text-center">
            <div className="text-xs text-muted-foreground">{formatWeekday(day)}</div>
            <div
              className={cn(
                "mx-auto flex size-7 items-center justify-center rounded-full text-sm font-medium",
                isSameDay(day, today) && "bg-primary text-primary-foreground",
              )}
            >
              {day.getDate()}
            </div>
            <div className="mt-1 space-y-0.5 empty:hidden">
              {eventsOnDay(events, day)
                .filter((e) => e.all_day)
                .map(chip)}
            </div>
          </div>
        ))}
      </div>

      <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto">
        <div className="grid" style={{ ...cols, height: HOUR_HEIGHT * 24 }}>
          <div>
            {HOURS.map((h) => (
              <div key={h} className="pr-2 text-right text-[11px] text-muted-foreground" style={{ height: HOUR_HEIGHT }}>
                {h > 0 && `${String(h).padStart(2, "0")}:00`}
              </div>
            ))}
          </div>
          {days.map((day) => {
            const dayStart = startOfDay(day);
            const dayEnd = addDays(dayStart, 1);
            return (
              <div
                key={day.toISOString()}
                className="relative border-l"
                onDoubleClick={(e) => slotFromClick(day, e)}
                style={{
                  backgroundImage: "linear-gradient(to bottom, var(--border) 1px, transparent 1px)",
                  backgroundSize: `100% ${HOUR_HEIGHT}px`,
                }}
              >
                {eventsOnDay(events, day)
                  .filter((e) => !e.all_day)
                  .map((e) => {
                    const { start, end } = eventInterval(e);
                    const top = ((Math.max(start.getTime(), dayStart.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    const bottom = ((Math.min(end.getTime(), dayEnd.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    return (
                      <button
                        key={e.id}
                        type="button"
                        onClick={() => onSelectEvent(e)}
                        onDoubleClick={(ev) => ev.stopPropagation()}
                        className="absolute inset-x-0.5 overflow-hidden rounded px-1.5 py-0.5 text-left text-xs text-white"
                        style={{
                          top,
                          height: Math.max(bottom - top, 18),
                          backgroundColor: calendarColor(calendars, e.calendar_id),
                        }}
                      >
                        <span className="font-medium">{e.title}</span>
                        <span className="ml-1 opacity-80">{formatTime(start)}</span>
                      </button>
                    );
                  })}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
