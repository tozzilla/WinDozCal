import type { CalendarView } from "@/types";
import { addDays, addMonths, getMonthGrid, startOfDay, startOfWeek } from "@/utils/date";
import { AGENDA_DAYS, AgendaView } from "./AgendaView";
import { DayView } from "./DayView";
import { MonthView } from "./MonthView";
import type { CalendarRenderer } from "./types";
import { WeekView } from "./WeekView";


export const renderers: Record<CalendarView, CalendarRenderer> = {
  day: {
    view: "day",
    label: "Giorno",
    Component: DayView,
    getRange: (d) => ({ start: startOfDay(d), end: addDays(startOfDay(d), 1) }),
    navigate: (d, dir) => addDays(d, dir),
  },
  week: {
    view: "week",
    label: "Settimana",
    Component: WeekView,
    getRange: (d) => ({ start: startOfWeek(d), end: addDays(startOfWeek(d), 7) }),
    navigate: (d, dir) => addDays(d, 7 * dir),
  },
  month: {
    view: "month",
    label: "Mese",
    Component: MonthView,
    getRange: (d) => {
      const grid = getMonthGrid(d);
      return { start: grid[0], end: addDays(grid[41], 1) };
    },
    navigate: (d, dir) => addMonths(d, dir),
  },
  agenda: {
    view: "agenda",
    label: "Agenda",
    Component: AgendaView,
    getRange: (d) => ({ start: startOfDay(d), end: addDays(startOfDay(d), AGENDA_DAYS) }),
    navigate: (d, dir) => addDays(d, AGENDA_DAYS * dir),
  },
};

export const viewOrder: CalendarView[] = ["day", "week", "month", "agenda"];
