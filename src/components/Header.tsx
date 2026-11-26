import { ChevronLeft, ChevronRight, PanelLeft, Plus, Search } from "lucide-react";
import { renderers, viewOrder } from "@/calendar/renderers";
import { useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { useToday } from "@/hooks/useToday";
import { usePreferences } from "@/providers/queries";
import type { CalendarView } from "@/types";
import { addDays, formatMonthYear, startOfWeek, type WeekStart } from "@/utils/date";
import { isoWeek } from "@/utils/isoWeek";

const iconButton =
  "grid size-8 place-items-center rounded-md border bg-card text-foreground outline-none hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring";

/**
 * Settimana ISO da mostrare accanto al mese (ADR 024): Giorno e Agenda quella della data, Settimana quella
 * del lunedì della settimana mostrata (come le righe del Mese), Mese quella di oggi se oggi è nel mese.
 */
export function headerWeek(view: CalendarView, date: Date, today: Date, weekStartsOn: WeekStart): number | null {
  switch (view) {
    case "week": {
      const start = startOfWeek(date, weekStartsOn);
      return isoWeek(weekStartsOn === 0 ? addDays(start, 1) : start).week;
    }
    case "month":
      return date.getFullYear() === today.getFullYear() && date.getMonth() === today.getMonth() ? isoWeek(today).week : null;
    default:
      return isoWeek(date).week;
  }
}

/** `<  >  Oggi  Ottobre 2026` + ricerca + selettore vista (PRD §6), nel linguaggio del Palinsesto. */
export function Header() {
  const t = useT();
  const { view, setView, currentDate, setCurrentDate, toggleSidebar, setSearchOpen } = useUiStore();
  const openNew = useEditorStore((s) => s.openNew);
  const { navigate } = renderers[view];
  const settings = usePreferences();
  const today = useToday();
  const week = settings.show_week_numbers ? headerWeek(view, currentDate, today, settings.week_start) : null;

  return (
    <header className="flex h-15 shrink-0 items-center gap-2 border-b bg-card px-3 xl:px-5">
      <button type="button" className={cn(iconButton, "mr-1 border-transparent")} aria-label={t("shell.header.toggleSidebar")} onClick={toggleSidebar}>
        <PanelLeft className="size-4" />
      </button>
      <button type="button" className={iconButton} aria-label={t("shell.header.previous")} onClick={() => setCurrentDate(navigate(currentDate, -1))}>
        <ChevronLeft className="size-4" />
      </button>
      <button type="button" className={iconButton} aria-label={t("shell.header.next")} onClick={() => setCurrentDate(navigate(currentDate, 1))}>
        <ChevronRight className="size-4" />
      </button>
      <button
        type="button"
        className="h-8 rounded-md border bg-card px-3 text-[12px] font-semibold outline-none hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => setCurrentDate(new Date())}
      >
        {t("shell.header.today")}
      </button>
      <h1 className="ml-3 text-[20px] leading-none font-bold tracking-[-0.01em] whitespace-nowrap xl:text-[25px]">{formatMonthYear(currentDate)}</h1>
      {week !== null && (
        <span
          title={t("calendar.weekNumberTitle", { week })}
          className="shrink-0 rounded-[3px] border px-1.5 py-0.5 text-[12px] font-semibold whitespace-nowrap text-muted-foreground tabular-nums"
        >
          {t("calendar.weekNumber", { week })}
        </span>
      )}

      <div className="ml-auto flex items-center gap-2 xl:gap-2.5">
        <button
          type="button"
          onClick={() => setSearchOpen(true)}
          aria-label={t("shell.header.searchAria")}
          className="flex h-8 items-center gap-2 rounded-md border bg-card px-2.5 text-[13px] xl:w-58 text-muted-foreground outline-none hover:border-muted-foreground/40 focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Search className="size-3.5" />
          <span className="hidden flex-1 text-left xl:inline">{t("shell.header.search")}</span>
          <kbd className="hidden font-sans text-[12px] xl:inline">Ctrl+K</kbd>
        </button>
        <div role="tablist" aria-label={t("shell.header.views")} className="flex h-8 overflow-hidden rounded-md border bg-card">
          {viewOrder.map((v) => (
            <button
              key={v}
              type="button"
              role="tab"
              aria-selected={v === view}
              onClick={() => setView(v)}
              className={cn(
                "px-2.5 text-[13px] font-semibold outline-none xl:px-3.5 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
                v === view ? "bg-primary text-primary-foreground" : "hover:bg-secondary",
              )}
            >
              {renderers[v].label}
            </button>
          ))}
        </div>
        <button
          type="button"
          aria-label={t("shell.header.newEvent")}
          onClick={() => openNew()}
          className="grid h-8 w-9 place-items-center rounded-md bg-primary text-primary-foreground outline-none hover:opacity-90 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          <Plus className="size-4" strokeWidth={2.5} />
        </button>
      </div>
    </header>
  );
}
