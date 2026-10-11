import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { CalendarView, ThemeMode } from "@/types";

interface UiState {
  view: CalendarView;
  currentDate: Date;
  sidebarOpen: boolean;
  theme: ThemeMode;
  /** Appearance › compact mode (PRD §34): scala oraria ridotta. */
  compact: boolean;
  settingsOpen: boolean;
  searchOpen: boolean;
  quickAddOpen: boolean;
  /** Messaggio breve non bloccante (errori di drag & drop, azioni non permesse). */
  notice: string | null;
  setView: (view: CalendarView) => void;
  setCurrentDate: (date: Date) => void;
  toggleSidebar: () => void;
  setTheme: (theme: ThemeMode) => void;
  setCompact: (compact: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  setSearchOpen: (open: boolean) => void;
  setQuickAddOpen: (open: boolean) => void;
  setNotice: (notice: string | null) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      view: "week", // vista predefinita (PRD §5)
      currentDate: new Date(),
      sidebarOpen: true,
      theme: "system",
      compact: false,
      settingsOpen: false,
      searchOpen: false,
      quickAddOpen: false,
      notice: null,
      setView: (view) => set({ view }),
      setCurrentDate: (currentDate) => set({ currentDate }),
      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
      setTheme: (theme) => set({ theme }),
      setCompact: (compact) => set({ compact }),
      setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
      setSearchOpen: (searchOpen) => set({ searchOpen }),
      setQuickAddOpen: (quickAddOpen) => set({ quickAddOpen }),
      setNotice: (notice) => set({ notice }),
    }),
    {
      name: "windozcal-ui",
      partialize: (s) => ({ view: s.view, sidebarOpen: s.sidebarOpen, theme: s.theme, compact: s.compact }),
    },
  ),
);
