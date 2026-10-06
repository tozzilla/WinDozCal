import { useEffect } from "react";
import { renderers } from "@/calendar/renderers";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import type { CalendarView } from "@/types";

const VIEW_KEYS: Record<string, CalendarView> = { d: "day", w: "week", m: "month", a: "agenda" };

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName));
}

/** Mappa scorciatoie PRD §25. I tasti singoli sono ignorati mentre si scrive in un campo. */
export function useKeyboardShortcuts() {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const ui = useUiStore.getState();
      const editor = useEditorStore.getState();
      const key = e.key.toLowerCase();

      if (e.key === "Escape") {
        if (editor.open) editor.close();
        else if (ui.searchOpen) ui.setSearchOpen(false);
        else if (ui.settingsOpen) ui.setSettingsOpen(false);
        return;
      }

      if (e.ctrlKey && !e.altKey && !e.shiftKey) {
        if (key === "n") editor.openNew();
        else if (key === "k") ui.setSearchOpen(true);
        else if (key === "t") ui.setCurrentDate(new Date());
        else return;
        e.preventDefault();
        return;
      }

      if (e.ctrlKey || e.altKey || e.metaKey || isTyping(e.target) || editor.open) return;

      if (key in VIEW_KEYS) {
        ui.setView(VIEW_KEYS[key]);
      } else if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
        ui.setCurrentDate(renderers[ui.view].navigate(ui.currentDate, e.key === "ArrowRight" ? 1 : -1));
      } else {
        return;
      }
      e.preventDefault();
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);
}
