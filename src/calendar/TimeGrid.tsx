import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import type { Event } from "@/types";
import { addDays, formatTime, formatWeekday, isSameDay, startOfDay, toIsoWithOffset } from "@/utils/date";
import { eventInterval, eventKey, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import { clickRange, slotRange } from "./drag";
import { layoutOverlaps } from "./layout";
import { broadcastMarks, eventSurface, MARK_LABEL, type BroadcastMark } from "./palinsesto";
import type { CalendarRendererProps } from "./types";
import { useEventDrag } from "./useEventDrag";

/** Scala oraria del palinsesto, identica in Giorno e Settimana (le viste restano a registro). */
const HOUR_HEIGHT = 72;
const HOURS = Array.from({ length: 24 }, (_, i) => i);
const MIN_EVENT_HEIGHT = 20;
/** Sotto questa altezza orario e titolo stanno su una riga sola. */
const ONE_LINE_HEIGHT = 44;
/** Senza oggi nel periodo mostrato la griglia parte da qui. */
const FIRST_HOUR = 8;

interface TimeGridProps extends CalendarRendererProps {
  days: Date[];
}

/** Data corrente, aggiornata ogni minuto (banda IN ONDA e segni del palinsesto). */
function useNow(intervalMs = 60_000) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const id = setInterval(() => setNow(new Date()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}

const isInteractive = (target: EventTarget) => !!(target as HTMLElement).closest("[data-event],button");

const MARK_CLASS: Record<BroadcastMark, string> = {
  onair: "bg-onair text-onair-foreground",
  next: "bg-primary text-primary-foreground",
  conflict: "bg-onair/12 text-onair ring-1 ring-onair/40",
};

/**
 * Eventi di tutto il giorno nella fascia sopra la griglia: una barra per evento, estesa sui giorni visibili
 * che copre, impilata nella prima riga libera.
 */
function allDayRows(events: Event[], days: Date[]) {
  const first = startOfDay(days[0]);
  const last = addDays(startOfDay(days[days.length - 1]), 1);
  const dayIndex = (d: Date) => Math.round((startOfDay(d).getTime() - first.getTime()) / 86_400_000);
  const rows: number[][] = [];
  return events
    .filter((e) => e.all_day)
    .map((e) => ({ e, ...eventInterval(e) }))
    .filter(({ start, end }) => start < last && end > first)
    .sort((a, b) => a.start.getTime() - b.start.getTime())
    .map(({ e, start, end }) => {
      const from = Math.max(0, dayIndex(start));
      const to = Math.min(days.length, Math.max(from + 1, dayIndex(new Date(end.getTime() - 1)) + 1));
      let row = rows.findIndex((taken) => taken.every((d) => d < from || d >= to));
      if (row === -1) row = rows.push([]) - 1;
      for (let d = from; d < to; d++) rows[row].push(d);
      return { e, from, span: to - from, row };
    });
}

/** Griglia oraria condivisa da Giorno e Settimana: palinsesto con banda IN ONDA, eventi affiancati, drag & drop. */
export function TimeGrid({ days, events, calendars, onSelectSlot, onSelectEvent, onChangeEventTime, onNotice }: TimeGridProps) {
  const now = useNow();
  const scroller = useRef<HTMLDivElement>(null);
  const columnsRef = useRef<HTMLDivElement>(null);
  const { drag, begin } = useEventDrag({ days, hourHeight: HOUR_HEIGHT, columnsRef, calendars, onSelectEvent, onChangeEventTime, onNotice });
  const cols = { gridTemplateColumns: `repeat(${days.length}, minmax(0, 1fr))` };

  // Se il periodo mostrato contiene oggi, la griglia si apre con la banda IN ONDA a un terzo dell'altezza;
  // altrimenti dalle 08:00. Si riposiziona solo per azioni dell'utente (apertura, cambio di vista o periodo,
  // finestra di nuovo visibile dal tray), mai mentre si lavora.
  const firstDay = days[0].getTime();
  useEffect(() => {
    const position = () => {
      const el = scroller.current;
      if (!el) return;
      const current = new Date();
      el.scrollTop = days.some((d) => isSameDay(d, current))
        ? ((current.getTime() - startOfDay(current).getTime()) / 3_600_000) * HOUR_HEIGHT - el.clientHeight / 3
        : FIRST_HOUR * HOUR_HEIGHT;
    };
    position();
    const onVisible = () => document.visibilityState === "visible" && position();
    document.addEventListener("visibilitychange", onVisible);
    return () => document.removeEventListener("visibilitychange", onVisible);
  }, [days.length, firstDay]);

  // Durante il drag l'evento appare già nella nuova posizione e il layout delle sovrapposizioni si ricalcola.
  const shown: Event[] = drag
    ? events.map((e) => (eventKey(e) === drag.key ? { ...e, start: toIsoWithOffset(drag.start), end: toIsoWithOffset(drag.end) } : e))
    : events;
  const marks = broadcastMarks(shown, calendars, now);

  // Clic su una fascia vuota: evento di un'ora; trascinamento: evento della durata disegnata (anteprima a 15 minuti).
  const [creating, setCreating] = useState<{ day: number; start: Date; end: Date } | null>(null);
  const beginCreate = (dayIndex: number, down: React.PointerEvent<HTMLDivElement>) => {
    if (down.button !== 0 || isInteractive(down.target)) return;
    down.preventDefault();
    const day = days[dayIndex];
    const top = down.currentTarget.getBoundingClientRect().top;
    const minutesAt = (clientY: number) => ((clientY - top) / HOUR_HEIGHT) * 60;
    const from = minutesAt(down.clientY);
    let moved = false;
    let range = clickRange(day, from);

    const onMove = (m: PointerEvent) => {
      if (!moved && Math.abs(m.clientY - down.clientY) < 4) return;
      moved = true;
      range = slotRange(day, from, minutesAt(m.clientY));
      setCreating({ day: dayIndex, ...range });
    };
    const finish = (commit: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      window.removeEventListener("keydown", onKey, true);
      setCreating(null);
      if (commit) onSelectSlot(range.start, range.end);
    };
    const onUp = () => finish(true);
    const onCancel = () => finish(false);
    const onKey = (k: KeyboardEvent) => {
      if (k.key !== "Escape") return;
      k.stopPropagation();
      finish(false);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
    window.addEventListener("keydown", onKey, true);
  };

  const allDay = allDayRows(shown, days);
  const todayVisible = days.some((day) => isSameDay(day, now));
  const nowTop = ((now.getTime() - startOfDay(now).getTime()) / 3_600_000) * HOUR_HEIGHT;

  return (
    <div className="flex h-full flex-col bg-grid">
      {/* Intestazioni: stesso scrollbar-gutter del corpo, così le colonne restano allineate. */}
      <div className="overflow-y-hidden border-b bg-card [scrollbar-gutter:stable]">
        <div className="flex">
          <div className="w-16 shrink-0" />
          <div className="grid flex-1" style={cols}>
            {days.map((day) => {
              const today = isSameDay(day, now);
              return (
                <div key={day.toISOString()} className="flex items-baseline gap-1.5 px-2.5 pt-2.5 pb-2">
                  <span className="text-[12px] font-semibold tracking-[0.08em] text-muted-foreground uppercase">{formatWeekday(day)}</span>
                  <span className={cn("text-[24px] leading-none font-bold tracking-[-0.02em]", today && "text-onair")} aria-current={today ? "date" : undefined}>
                    {day.getDate()}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
        {allDay.length > 0 && (
          <div className="flex border-t bg-background">
            <div className="w-16 shrink-0" />
            <div className="grid flex-1 gap-y-0.5 p-0.5" style={cols}>
              {allDay.map(({ e, from, span, row }) => (
                <button
                  key={eventKey(e)}
                  type="button"
                  onClick={() => onSelectEvent(e)}
                  className="truncate rounded-sm px-2 py-0.5 text-left text-[12px] font-semibold outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  style={{ gridColumn: `${from + 1} / span ${span}`, gridRow: row + 1, ...eventSurface(calendarColor(calendars, e.calendar_id), e.status === "free") }}
                >
                  {e.title}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto [scrollbar-gutter:stable]">
        <div className="relative flex" style={{ height: HOUR_HEIGHT * 24 }}>
          <div className="w-16 shrink-0">
            {HOURS.map((h) => (
              <div key={h} className="pt-0.5 pr-2 text-right text-[15px] leading-none font-bold" style={{ height: HOUR_HEIGHT }}>
                {`${String(h).padStart(2, "0")}:00`}
              </div>
            ))}
          </div>
          <div ref={columnsRef} className="tg-cols grid flex-1" style={cols}>
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
                  className={cn("tg-col relative border-l border-grid-line", isSameDay(day, now) && "bg-today")}
                  onPointerDown={(down) => beginCreate(days.indexOf(day), down)}
                  style={{
                    backgroundImage: "linear-gradient(to bottom, var(--grid-line) 1px, transparent 1px)",
                    backgroundSize: `100% ${HOUR_HEIGHT}px`,
                  }}
                >
                  {creating?.day === days.indexOf(day) && (
                    <div
                      aria-hidden
                      className="pointer-events-none absolute inset-x-1 z-20 rounded-[4px] border-2 border-dashed border-primary/60 bg-primary/8 px-2 pt-1 text-[13px] font-bold"
                      style={{
                        top: ((creating.start.getTime() - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT + 1,
                        height: ((creating.end.getTime() - creating.start.getTime()) / 3_600_000) * HOUR_HEIGHT - 3,
                      }}
                    >
                      {formatTime(creating.start)}–{formatTime(creating.end)}
                    </div>
                  )}
                  {timed.map((e) => {
                    const key = eventKey(e);
                    const { start, end } = eventInterval(e);
                    const top = ((Math.max(start.getTime(), dayStart.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    const bottom = ((Math.min(end.getTime(), dayEnd.getTime()) - dayStart.getTime()) / 3_600_000) * HOUR_HEIGHT;
                    const height = Math.max(bottom - top - 3, MIN_EVENT_HEIGHT);
                    const { column, columns } = layout.get(key) ?? { column: 0, columns: 1 };
                    const dragging = drag?.key === key;
                    const mark = marks.get(key);
                    const oneLine = height < ONE_LINE_HEIGHT;
                    return (
                      <div
                        key={key}
                        data-event
                        role="button"
                        tabIndex={0}
                        aria-label={`${e.title}, ${formatTime(start)}–${formatTime(end)}${mark ? `, ${MARK_LABEL[mark].toLowerCase()}` : ""}`}
                        onPointerDown={(down) => begin(e, "move", down)}
                        onKeyDown={(k) => k.key === "Enter" && onSelectEvent(e)}
                        className={cn(
                          "@container absolute cursor-grab touch-none overflow-hidden rounded-[4px] px-2 pt-[5px] pb-1 text-left text-[12.5px] leading-[1.3] text-foreground outline-none select-none focus-visible:ring-2 focus-visible:ring-ring",
                          mark === "onair" && "ring-2 ring-onair",
                          dragging && "z-20 cursor-grabbing opacity-90 shadow-[0_8px_22px_rgb(16_20_42/0.22)]",
                        )}
                        style={
                          {
                            top: top + 1,
                            height,
                            // Affiancati nelle colonne larghe, a cascata in quelle strette (vedi .tg-col in index.css).
                            "--side-left": `${(column / columns) * 100}%`,
                            "--side-width": `${100 / columns}%`,
                            "--cascade-left": columns > 1 ? `${(column * 40) / (columns - 1)}%` : "0%",
                            ...eventSurface(calendarColor(calendars, e.calendar_id), e.status === "free"),
                          } as React.CSSProperties
                        }
                      >
                        {mark && !oneLine && (
                          <span className={cn("mb-1 inline-block max-w-full truncate rounded-[3px] px-1 py-px align-top text-[10px] font-bold tracking-[0.05em] @max-[96px]:hidden", MARK_CLASS[mark])}>
                            {MARK_LABEL[mark]}
                          </span>
                        )}
                        {oneLine ? (
                          <div className="truncate">
                            <span className="font-bold">{formatTime(start)}</span> <span className="font-semibold">{e.title}</span>
                          </div>
                        ) : (
                          <>
                            <div className="flex items-center gap-1 text-[13px] font-bold text-foreground/75">
                              {mark && <span aria-hidden className={cn("hidden size-2 shrink-0 rounded-full @max-[96px]:inline-block", mark === "next" ? "bg-primary" : "bg-onair")} />}
                              <span>
                                {formatTime(start)}
                                <span className="@max-[96px]:hidden">–{formatTime(end)}</span>
                              </span>
                            </div>
                            <div className="line-clamp-3 font-semibold break-words @max-[96px]:text-[11px]">{e.title}</div>
                          </>
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
                </div>
              );
            })}
          </div>
          {todayVisible && (
            <div aria-hidden className="pointer-events-none absolute inset-x-0 z-10" style={{ top: nowTop }}>
              <div className="absolute right-0 left-16 h-[3px] -translate-y-1/2 bg-onair" />
              <span className="absolute left-0.5 flex -translate-y-1/2 items-center gap-1 rounded-[3px] bg-onair px-1.5 py-0.5 text-[10px] font-bold tracking-[0.06em] whitespace-nowrap text-onair-foreground">
                <span className="onair-tally size-1.5 rounded-full bg-onair-foreground" />
                IN ONDA {formatTime(now)}
              </span>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
