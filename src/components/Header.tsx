import { ChevronLeft, ChevronRight, PanelLeft, Plus, Search } from "lucide-react";
import { renderers, viewOrder } from "@/calendar/renderers";
import { cn } from "@/lib/utils";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { formatMonthYear } from "@/utils/date";

const iconButton =
  "grid size-8 place-items-center rounded-md border bg-card text-foreground outline-none hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring";

/** `<  >  Oggi  Ottobre 2026` + ricerca + selettore vista (PRD §6), nel linguaggio del Palinsesto. */
export function Header() {
  const { view, setView, currentDate, setCurrentDate, toggleSidebar, setSearchOpen } = useUiStore();
  const openNew = useEditorStore((s) => s.openNew);
  const { navigate } = renderers[view];

  return (
    <header className="flex h-15 shrink-0 items-center gap-2 border-b bg-card px-5">
      <button type="button" className={cn(iconButton, "mr-1 border-transparent")} aria-label="Mostra o nascondi la barra laterale" onClick={toggleSidebar}>
        <PanelLeft className="size-4" />
      </button>
      <button type="button" className={iconButton} aria-label="Periodo precedente" onClick={() => setCurrentDate(navigate(currentDate, -1))}>
        <ChevronLeft className="size-4" />
      </button>
      <button type="button" className={iconButton} aria-label="Periodo successivo" onClick={() => setCurrentDate(navigate(currentDate, 1))}>
        <ChevronRight className="size-4" />
      </button>
      <button
        type="button"
        className="h-8 rounded-md border bg-card px-3 text-[12px] font-semibold outline-none hover:bg-secondary focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => setCurrentDate(new Date())}
      >
        Oggi
      </button>
      <h1 className="ml-3 text-[25px] leading-none font-bold tracking-[-0.01em]">{formatMonthYear(currentDate)}</h1>

      <div className="ml-auto flex items-center gap-2.5">
        <button
          type="button"
          onClick={() => setSearchOpen(true)}
          className="flex h-8 w-58 items-center gap-2 rounded-md border bg-card px-2.5 text-[13px] text-muted-foreground outline-none hover:border-muted-foreground/40 focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Search className="size-3.5" />
          <span className="flex-1 text-left">Cerca eventi</span>
          <kbd className="font-sans text-[12px]">Ctrl+K</kbd>
        </button>
        <div role="tablist" aria-label="Vista" className="flex h-8 overflow-hidden rounded-md border bg-card">
          {viewOrder.map((v) => (
            <button
              key={v}
              type="button"
              role="tab"
              aria-selected={v === view}
              onClick={() => setView(v)}
              className={cn(
                "px-3.5 text-[13px] font-semibold outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
                v === view ? "bg-primary text-primary-foreground" : "hover:bg-secondary",
              )}
            >
              {renderers[v].label}
            </button>
          ))}
        </div>
        <button
          type="button"
          aria-label="Nuovo evento"
          onClick={() => openNew()}
          className="grid h-8 w-9 place-items-center rounded-md bg-primary text-primary-foreground outline-none hover:opacity-90 focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          <Plus className="size-4" strokeWidth={2.5} />
        </button>
      </div>
    </header>
  );
}
