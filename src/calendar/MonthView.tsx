import { useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { addDays, dayKey, formatWeekday, getMonthGrid, getWeekDays, isSameDay } from "@/utils/date";
import { isoWeek } from "@/utils/isoWeek";
import { eventKey, eventsOnDay } from "@/utils/events";
import { calendarColor } from "./colors";
import { EventIcon, eventSurface } from "./appearance";
import { HolidayLabel, isHoliday, MoonMark } from "./DayMarks";
import type { CalendarRendererProps } from "./types";

const MAX_CHIPS = 3;
/** Colonna del numero di settimana (ADR 024), uguale in intestazione e corpo. */
const WEEK_COL = "2rem";

/** Vista mensile compatta (PRD §5): al massimo MAX_CHIPS eventi per cella, poi "+N altri" porta al giorno. */
export function MonthView({ date, prefs, events, calendars, annotations, onSelectSlot, onSelectEvent, onShowDay }: CalendarRendererProps) {
  const t = useT();
  const today = new Date();
  const grid = getMonthGrid(date, prefs.weekStartsOn);
  const weeks = prefs.showWeekNumbers;
  const columns = { gridTemplateColumns: weeks ? `${WEEK_COL} repeat(7, minmax(0, 1fr))` : "repeat(7, minmax(0, 1fr))" };

  return (
    <div className="flex h-full flex-col bg-grid">
      <div className="grid border-b bg-card" style={columns}>
        {weeks && <div />}
        {getWeekDays(date, prefs.weekStartsOn).map((d) => (
          <div key={d.getDay()} className="px-2.5 py-2 text-[12px] font-semibold tracking-[0.08em] text-muted-foreground uppercase">
            {formatWeekday(d)}
          </div>
        ))}
      </div>
      <div className="grid min-h-0 flex-1 grid-rows-6" style={columns}>
        {grid.flatMap((day, i) => {
          const annotation = annotations.get(dayKey(day));
          // Con una festività la cella cede una riga di eventi, così "+N altri" resta visibile.
          const maxChips = isHoliday(annotation) ? MAX_CHIPS - 1 : MAX_CHIPS;
          const dayEvents = eventsOnDay(events, day);
          const hidden = dayEvents.length - maxChips;
          // Numerazione ISO: la riga conta dal suo lunedì, anche quando la settimana parte di domenica.
          const week = weeks && i % 7 === 0 ? isoWeek(prefs.weekStartsOn === 0 ? addDays(day, 1) : day).week : null;
          const cells = [];
          if (week !== null) {
            cells.push(
              <div
                key={`w${i}`}
                title={t("calendar.weekNumberTitle", { week })}
                className="border-b border-grid-line pt-2 text-center text-[11px] font-semibold text-muted-foreground tabular-nums"
              >
                {week}
              </div>,
            );
          }
          cells.push(
            <div
              key={day.toISOString()}
              className={cn("min-h-0 overflow-hidden border-b border-l border-grid-line p-1.5", isSameDay(day, today) && "bg-today")}
              onDoubleClick={(e) => {
                if ((e.target as HTMLElement).closest("button")) return;
                // Doppio clic sul giorno: evento all'inizio dell'orario di lavoro, con la durata predefinita.
                const start = new Date(day.getFullYear(), day.getMonth(), day.getDate(), 0, prefs.workStart);
                onSelectSlot(start, new Date(start.getTime() + prefs.defaultDuration * 60_000));
              }}
            >
              <div className="mb-1 flex items-center gap-1 px-0.5">
                <span
                  className={cn(
                    "text-[15px] leading-none font-bold",
                    day.getMonth() !== date.getMonth() && "font-semibold text-muted-foreground/70",
                    isHoliday(annotation) && "text-holiday",
                    isSameDay(day, today) && "text-onair",
                  )}
                >
                  {day.getDate()}
                </span>
                <MoonMark annotation={annotation} className="ml-auto" />
              </div>
              <HolidayLabel annotation={annotation} className="mb-0.5 px-0.5" />
              {dayEvents.slice(0, maxChips).map((e) => (
                <button
                  key={eventKey(e)}
                  type="button"
                  onClick={() => onSelectEvent(e)}
                  className="mb-0.5 flex w-full items-center truncate rounded-[3px] px-1.5 py-px text-left text-[12px] font-semibold outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  style={{ ...eventSurface(calendarColor(calendars, e.calendar_id), e.status === "free", e), borderLeftWidth: 3 }}
                >
                  <EventIcon name={e.icon} className="mr-1 size-3 shrink-0" />
                  <span className="truncate">{e.title}</span>
                </button>
              ))}
              {hidden > 0 && (
                <button type="button" onClick={() => onShowDay(day)} className="px-1 text-[12px] font-semibold text-muted-foreground hover:text-foreground">
                  {t("calendar.month.more", { count: hidden })}
                </button>
              )}
            </div>,
          );
          return cells;
        })}
      </div>
    </div>
  );
}
