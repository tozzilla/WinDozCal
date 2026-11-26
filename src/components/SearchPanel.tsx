import { useEffect, useState } from "react";
import { Search } from "lucide-react";
import { calendarColor } from "@/calendar/colors";
import { dateFormat, useT } from "@/i18n";
import { useCalendars, useSearchEvents } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import type { Event } from "@/types";
import { formatTime, parseEventDate } from "@/utils/date";

function when(e: Event): string {
  const start = parseEventDate(e.start);
  const day = dateFormat({ weekday: "short", day: "numeric", month: "short", year: "numeric" }).format(start);
  return e.all_day ? day : `${day} · ${formatTime(start)}`;
}

/** Ricerca locale `Ctrl + K` (PRD §24): titolo, descrizione, luogo e partecipanti, senza rete. */
export function SearchPanel() {
  const t = useT();
  const { searchOpen, setSearchOpen, setCurrentDate } = useUiStore();
  const openExisting = useEditorStore((s) => s.openExisting);
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [active, setActive] = useState(0);
  const { data: results = [], isFetching } = useSearchEvents(debounced);
  const { data: calendars = [] } = useCalendars();

  useEffect(() => {
    const timer = setTimeout(() => setDebounced(query), 120);
    return () => clearTimeout(timer);
  }, [query]);
  useEffect(() => {
    setActive(0);
  }, [debounced]);
  useEffect(() => {
    if (!searchOpen) setQuery("");
  }, [searchOpen]);

  if (!searchOpen) return null;

  const choose = (e: Event) => {
    setSearchOpen(false);
    setCurrentDate(parseEventDate(e.start));
    openExisting(e);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") setActive((i) => Math.min(i + 1, results.length - 1));
    else if (e.key === "ArrowUp") setActive((i) => Math.max(i - 1, 0));
    else if (e.key === "Enter" && results[active]) choose(results[active]);
    else return;
    e.preventDefault();
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-rail/35 p-4 pt-[12vh]" onClick={(e) => e.target === e.currentTarget && setSearchOpen(false)}>
      <div role="dialog" aria-label={t("shell.search.title")} className="h-fit w-full max-w-xl overflow-hidden rounded-lg border bg-popover text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]">
        <div className="flex items-center gap-2 border-b px-3">
          <Search className="size-4 text-muted-foreground" />
          <input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={t("shell.search.placeholder")}
            className="w-full bg-transparent py-3.5 text-[16px] outline-none placeholder:text-muted-foreground/80"
          />
        </div>
        {debounced.trim() && (
          <ul role="listbox" className="max-h-[50vh] overflow-y-auto p-1">
            {results.map((e, i) => (
              <li key={e.id} role="option" aria-selected={i === active}>
                <button
                  type="button"
                  onMouseEnter={() => setActive(i)}
                  onClick={() => choose(e)}
                  className={`flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm ${i === active ? "bg-accent" : ""}`}
                >
                  <span className="h-8 w-1.5 shrink-0 rounded-[1px]" style={{ background: calendarColor(calendars, e.calendar_id) }} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-semibold">{e.title || t("shell.search.untitled")}</span>
                    {e.location && <span className="block truncate text-xs text-muted-foreground">{e.location}</span>}
                  </span>
                  <span className="shrink-0 text-[12px] font-semibold text-muted-foreground">{when(e)}</span>
                </button>
              </li>
            ))}
            {results.length === 0 && !isFetching && <li className="px-3 py-2 text-sm text-muted-foreground">{t("shell.search.empty")}</li>}
          </ul>
        )}
      </div>
    </div>
  );
}
