import { create } from "zustand";
import type { Event } from "@/types";

/** Stato dell'editor evento: `event` valorizzato = modifica, altrimenti nuovo evento con `slot` opzionale. */
interface EditorState {
  open: boolean;
  /** Quando si è aperto: serve a ignorare il secondo clic di un doppio clic che cadrebbe sullo sfondo. */
  openedAt: number;
  event: Event | null;
  slot: { start: Date; end: Date } | null;
  openNew: (slot?: { start: Date; end: Date }) => void;
  openExisting: (event: Event) => void;
  close: () => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  open: false,
  openedAt: 0,
  event: null,
  slot: null,
  openNew: (slot) => set({ open: true, openedAt: Date.now(), event: null, slot: slot ?? null }),
  openExisting: (event) => set({ open: true, openedAt: Date.now(), event, slot: null }),
  close: () => set({ open: false, event: null, slot: null }),
}));
