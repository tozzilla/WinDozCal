import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import type { Event } from "@/types";
import { addDays, formatTime, formatWeekday, isSameDay, startOfDay, toIsoWithOffset } from "@/utils/date";
import { eventInterval, eventKey, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import { layoutOverlaps } from "./layout";
import type { CalendarRendererProps } from "./types";
import { useEventDrag } from "./useEventDrag";

const HOUR_HEIGHT = 48;
const HOURS = Array.from({ length: 24 }, (_, i) => i);
const MIN_EVENT_HEIGHT = 18;

interface TimeGridProps extends CalendarRendererProps {
  days: Date[];
}

/** Data corrente, aggiornata ogni minuto (per l'indicatore dell'ora). */
function useNow(intervalMs = 60_000) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

const isInteractive = (target: EventTarget) => !!(target as HTMLElement).closest("[data-event],button");

/** Griglia oraria condivisa da Giorno e Settimana: eventi affiancati, ora corrente, drag & drop. */
export function TimeGrid({ days, events, calendars, onSelectSlot, onSelectEvent, onChangeEventTime, onNotice }: TimeGridProps) {
  const now = useNow();
  const scroller = useRef<HTMLDivElement>(null);
  const columnsRef = useRef<HTMLDivElement>(null);
  const { drag, begin } = useEventDrag({ days, hourHeight: HOUR_HEIGHT, columnsRef, calendars, onSelectEvent, onChangeEventTime, onNotice });
  const cols = { gridTemplateColumns: `repeat(${days.length}, minmax(0, 1fr))` };

  // All'apertura la griglia parte dalle 7:00 invece che da mezzanotte.
  useEffect(() => {
    if (scroller.current) scroller.current.scrollTop = 7 * HOUR_HEIGHT;
  }, [days.length]);

  // Durante il drag l'evento appare già nella nuova posizione e il layout delle sovrapposizioni si ricalcola.
  const shown: Event[] = drag
    ? events.map((e) => (eventKey(e) === drag.key ? { ...e, start: toIsoWithOffset(drag.start), end: toIsoWithOffset(drag.end) } : e))
    : events;

  const slotFromClick = (day: Date, e: React.MouseEvent<HTMLDivElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const hour = Math.max(0, Math.min(23, Math.floor((e.clientY - rect.top) / HOUR_HEIGHT)));
    const start = new Date(day.getFullYear(), day.getMonth(), day.getDate(), hour);
    onSelectSlot(start, new Date(start.getTime() + 60 * 60_000));
  };

  const hasAllDay = days.some((day) => eventsOnDay(shown, day).some((e) => e.all_day));

  return (
    <div className="flex h-full flex-col">
      {/* Intestazioni: stesso scrollbar-gutter del corpo, così le colonne restano allineate. */}
      <div className="overflow-y-hidden border-b [scrollbar-gutter:stable]">
        <div className="flex">
          <div className="w-14 shrink-0" />
          <div className="grid flex-1" style={cols}>
            {days.map((day) => (
              <div key={day.toISOString()} className="border-l px-2 py-1.5 text-center">
                <div className="text-xs text-muted-foreground">{formatWeekday(day)}</div>
                <div
                  className={cn(
                    "mx-auto flex size-7 items-center justify-center rounded-full text-sm font-medium",
                    isSameDay(day, now) && "bg-primary text-primary-foreground",
                  )}
                >
                  {day.getDate()}
                </div>
              </div>
            ))}
          </div>
        </div>
        {hasAllDay && (
          <div className="flex border-t">
            <div className="flex w-14 shrink-0 items-center justify-end pr-2 text-[10px] text-muted-foreground">Tutto il giorno</div>
            <div className="grid flex-1" style={cols}>
              {days.map((day) => (
                <div key={day.toISOString()} className="space-y-0.5 border-l p-0.5">
                  {eventsOnDay(shown, day)
                    .filter((e) => e.all_day)
                    .map((e) => (
                      <button
                        key={eventKey(e)}
                        type="button"
                        onClick={() => onSelectEvent(e)}
                        className="block w-full truncate rounded px-1.5 py-0.5 text-left text-xs font-medium text-white"
                        style={{ backgroundColor: calendarColor(calendars, e.calendar_id) }}
                      >
                        {e.title}
                      </button>
                    ))}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto [scrollbar-gutter:stable]">
        <div className="flex" style={{ height: HOUR_HEIGHT * 24 }}>
          <div className="w-14 shrink-0">
            {HOURS.map((h) => (
              <div key={h} className="pr-2 text-right text-[11px] text-muted-foreground" style={{ height: HOUR_HEIGHT }}>
                {h > 0 && `${String(h).padStart(2, "0")}:00`}
              </div>
            ))}
          </div>
          <div ref={columnsRef} className="grid flex-1" style={cols}>
            {days.map((day) => {
              const dayStart = startOfDay(day);
              const dayEnd = addDays(dayStart, 1);
              const timed = eventsOnDay(shown, day).filter((e) => !e.all_day);
              const layout = layoutOverlaps(
                timed.map((e) => {
                  const { start, end } = eventInterval(e);
                  return {
                    key: eventKey(e),
                    start: Math.max(start.getTime(), dayStart.getTime()),
                    end: Math.max(Math.min(end.getTime(), dayEnd.getTime()), start.getTime() + 15 * 60_000),
                  };
                }),
              );
              return (
                <div
                  key={day.toISOString()}
                  className="relative border-l"
                  onDoubleClick={(e) => {
                    if (!isInteractive(e.target)) slotFromClick(day, e);
                  }}
                  style={{
                    backgroundImage: "linear-gradient(to bottom, var(--border) 1px, transparent 1px)",
                    backgroundSize: `100% ${HOUR_HEIGHT}px`,
                  }}
                >
                  {timed.map((e) => {
                    const key = eventKey(e);
                    const { start, end } = eventInterval(e);
                    const top = ((Math.max(start.getTime(), dayStart.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    const bottom = ((Math.min(end.getTime(), dayEnd.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    const height = Math.max(bottom - top, MIN_EVENT_HEIGHT);
                    const { column, columns } = layout.get(key) ?? { column: 0, columns: 1 };
                    const dragging = drag?.key === key;
                    return (
                      <div
                        key={key}
                        data-event
                        role="button"
                        tabIndex={0}
                        onPointerDown={(down) => begin(e, "move", down)}
                        onKeyDown={(k) => k.key === "Enter" && onSelectEvent(e)}
                        className={cn(
                          "absolute cursor-grab touch-none select-none overflow-hidden rounded px-1.5 py-0.5 text-left text-xs text-white outline-none focus-visible:ring-2 focus-visible:ring-ring",
                          dragging && "z-20 cursor-grabbing opacity-90 shadow-lg",
                        )}
                        style={{
                          top,
                          height,
                          left: `calc(${(column / columns) * 100}% + 1px)`,
                          width: `calc(${100 / columns}% - 3px)`,
                          backgroundColor: calendarColor(calendars, e.calendar_id),
                        }}
                      >
                        <div className="truncate font-medium">{e.title}</div>
                        {height >= 34 && (
                          <div className="truncate opacity-80">
                            {formatTime(start)}–{formatTime(end)}
                          </div>
                        )}
                        <div
                          aria-hidden
                          onPointerDown={(down) => {
                            down.stopPropagation();
                            begin(e, "resize", down);
                          }}
                          className="absolute inset-x-0 bottom-0 h-1.5 cursor-ns-resize"
                        />
                      </div>
                    );
                  })}
                  {isSameDay(day, now) && (
                    <div
                      aria-hidden
                      className="pointer-events-none absolute inset-x-0 z-10 h-px bg-destructive"
                      style={{ top: ((now.getTime() - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT }}
                    >
                      <span className="absolute -top-[3px] -left-1 size-[7px] rounded-full bg-destructive" />
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
