import { t } from "@/i18n";
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
    get label() {
      return t("calendar.views.day");
    },
    Component: DayView,
    getRange: (d) => ({ start: startOfDay(d), end: addDays(startOfDay(d), 1) }),
    navigate: (d, dir) => addDays(d, dir),
  },
  week: {
    view: "week",
    get label() {
      return t("calendar.views.week");
    },
    Component: WeekView,
    getRange: (d, p) => ({ start: startOfWeek(d, p.weekStartsOn), end: addDays(startOfWeek(d, p.weekStartsOn), 7) }),
    navigate: (d, dir) => addDays(d, 7 * dir),
  },
  month: {
    view: "month",
    get label() {
      return t("calendar.views.month");
    },
    Component: MonthView,
    getRange: (d, p) => {
      const grid = getMonthGrid(d, p.weekStartsOn);
      return { start: grid[0], end: addDays(grid[41], 1) };
    },
    navigate: (d, dir) => addMonths(d, dir),
  },
  agenda: {
    view: "agenda",
    get label() {
      return t("calendar.views.agenda");
    },
    Component: AgendaView,
    getRange: (d) => ({ start: startOfDay(d), end: addDays(startOfDay(d), AGENDA_DAYS) }),
    navigate: (d, dir) => addDays(d, AGENDA_DAYS * dir),
  },
};

export const viewOrder: CalendarView[] = ["day", "week", "month", "agenda"];
