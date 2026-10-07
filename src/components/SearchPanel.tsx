import { useEffect, useState } from "react";
import { Search } from "lucide-react";
import { calendarColor } from "@/calendar/colors";
import { useCalendars, useSearchEvents } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import type { Event } from "@/types";
import { formatTime } from "@/utils/date";

const dateFormat = new Intl.DateTimeFormat("it-IT", { weekday: "short", day: "numeric", month: "short", year: "numeric" });

function when(e: Event): string {
  const start = new Date(e.start);
  return e.all_day ? dateFormat.format(start) : `${dateFormat.format(start)} · ${formatTime(start)}`;
}

/** Ricerca locale `Ctrl + K` (PRD §24): titolo, descrizione, luogo e partecipanti, senza rete. */
export function SearchPanel() {
  const { searchOpen, setSearchOpen, setCurrentDate } = useUiStore();
  const openExisting = useEditorStore((s) => s.openExisting);
  const [query, setQuery] = useState("");
  const [debounced, setDebounced] = useState("");
  const [active, setActive] = useState(0);
  const { data: results = [], isFetching } = useSearchEvents(debounced);
  const { data: calendars = [] } = useCalendars();

  useEffect(() => {
    const t = setTimeout(() => setDebounced(query), 120);
    return () => clearTimeout(t);
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
    setCurrentDate(new Date(e.start));
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
    <div className="fixed inset-0 z-50 flex justify-center bg-black/30 p-4 pt-[12vh]" onClick={(e) => e.target === e.currentTarget && setSearchOpen(false)}>
      <div role="dialog" aria-label="Cerca eventi" className="h-fit w-full max-w-xl overflow-hidden rounded-xl border bg-popover text-popover-foreground shadow-xl">
        <div className="flex items-center gap-2 border-b px-3">
          <Search className="size-4 text-muted-foreground" />
          <input
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Cerca per titolo, luogo, descrizione o partecipante"
            className="w-full bg-transparent py-3 text-sm outline-none"
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
                  <span className="size-2.5 shrink-0 rounded-full" style={{ background: calendarColor(calendars, e.calendar_id) }} />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate font-medium">{e.title || "(senza titolo)"}</span>
                    {e.location && <span className="block truncate text-xs text-muted-foreground">{e.location}</span>}
                  </span>
                  <span className="shrink-0 text-xs text-muted-foreground">{when(e)}</span>
                </button>
              </li>
            ))}
            {results.length === 0 && !isFetching && <li className="px-3 py-2 text-sm text-muted-foreground">Nessun evento trovato.</li>}
          </ul>
        )}
      </div>
    </div>
  );
}
