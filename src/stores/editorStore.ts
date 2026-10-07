import { create } from "zustand";
import type { Event } from "@/types";

/** Valori iniziali di un nuovo evento oltre allo slot (bozza passata da Quick Add). */
export interface NewEventPreset {
  title: string;
  allDay: boolean;
}

/** Stato dell'editor evento: `event` valorizzato = modifica, altrimenti nuovo evento con `slot` e `preset` opzionali. */
interface EditorState {
  open: boolean;
  /** Quando si è aperto: serve a ignorare il secondo clic di un doppio clic che cadrebbe sullo sfondo. */
  openedAt: number;
  event: Event | null;
  slot: { start: Date; end: Date } | null;
  preset: NewEventPreset | null;
  openNew: (slot?: { start: Date; end: Date }, preset?: NewEventPreset) => void;
  openExisting: (event: Event) => void;
  close: () => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  open: false,
  openedAt: 0,
  event: null,
  slot: null,
  preset: null,
  openNew: (slot, preset) => set({ open: true, openedAt: Date.now(), event: null, slot: slot ?? null, preset: preset ?? null }),
  openExisting: (event) => set({ open: true, openedAt: Date.now(), event, slot: null, preset: null }),
  close: () => set({ open: false, event: null, slot: null, preset: null }),
}));
