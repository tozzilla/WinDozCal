import { ChevronLeft, ChevronRight, PanelLeft, Plus, Search } from "lucide-react";
import { renderers, viewOrder } from "@/calendar/renderers";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { formatMonthYear } from "@/utils/date";

/** `<  >  Oggi  Ottobre 2026` + selettore vista (PRD §6). */
export function Header() {
  const { view, setView, currentDate, setCurrentDate, toggleSidebar, setSearchOpen } = useUiStore();
  const openNew = useEditorStore((s) => s.openNew);
  const { navigate } = renderers[view];

  return (
    <header className="flex items-center gap-2 border-b px-3 py-2">
      <Button variant="ghost" size="icon" aria-label="Mostra o nascondi la barra laterale" onClick={toggleSidebar}>
        <PanelLeft />
      </Button>
      <Button variant="ghost" size="icon" aria-label="Periodo precedente" onClick={() => setCurrentDate(navigate(currentDate, -1))}>
        <ChevronLeft />
      </Button>
      <Button variant="ghost" size="icon" aria-label="Periodo successivo" onClick={() => setCurrentDate(navigate(currentDate, 1))}>
        <ChevronRight />
      </Button>
      <Button variant="outline" onClick={() => setCurrentDate(new Date())}>
        Oggi
      </Button>
      <h1 className="ml-2 text-lg font-semibold">{formatMonthYear(currentDate)}</h1>

      <div className="ml-auto flex items-center gap-2">
        <Button variant="ghost" size="icon" aria-label="Cerca (Ctrl+K)" onClick={() => setSearchOpen(true)}>
          <Search />
        </Button>
        <div className="flex rounded-lg border p-0.5">
          {viewOrder.map((v) => (
            <button
              key={v}
              type="button"
              onClick={() => setView(v)}
              className={cn("rounded-md px-3 py-1 text-sm", v === view ? "bg-primary text-primary-foreground" : "hover:bg-accent")}
            >
              {renderers[v].label}
            </button>
          ))}
        </div>
        <Button size="icon" aria-label="Nuovo evento" onClick={() => openNew()}>
          <Plus />
        </Button>
      </div>
    </header>
  );
}
