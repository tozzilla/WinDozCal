import { startOfDay } from "@/utils/date";
import { TimeGrid } from "./TimeGrid";
import type { CalendarRendererProps } from "./types";

/** Timeline verticale della giornata (PRD §5). */
export function DayView(props: CalendarRendererProps) {
  return <TimeGrid {...props} days={[startOfDay(props.date)]} />;
}
