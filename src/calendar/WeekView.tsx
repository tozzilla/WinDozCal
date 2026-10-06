import { getWeekDays } from "@/utils/date";
import { TimeGrid } from "./TimeGrid";
import type { CalendarRendererProps } from "./types";

/** Vista predefinita (PRD §5): 7 colonne, una per giorno, griglia oraria. */
export function WeekView(props: CalendarRendererProps) {
  return <TimeGrid {...props} days={getWeekDays(props.date)} />;
}
