import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { CalendarView, ThemeMode } from "@/types";

interface UiState {
  view: CalendarView;
  currentDate: Date;
  sidebarOpen: boolean;
  theme: ThemeMode;
  settingsOpen: boolean;
  searchOpen: boolean;
  setView: (view: CalendarView) => void;
  setCurrentDate: (date: Date) => void;
  toggleSidebar: () => void;
  setTheme: (theme: ThemeMode) => void;
  setSettingsOpen: (open: boolean) => void;
  setSearchOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      view: "week", // vista predefinita (PRD §5)
      currentDate: new Date(),
      sidebarOpen: true,
      theme: "system",
      settingsOpen: false,
      searchOpen: false,
      setView: (view) => set({ view }),
      setCurrentDate: (currentDate) => set({ currentDate }),
      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
      setTheme: (theme) => set({ theme }),
      setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
      setSearchOpen: (searchOpen) => set({ searchOpen }),
    }),
    {
      name: "windozcal-ui",
      partialize: (s) => ({ view: s.view, sidebarOpen: s.sidebarOpen, theme: s.theme }),
    },
  ),
);
