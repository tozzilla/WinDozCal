import type { ComponentType } from "react";
import type { Calendar, CalendarView, Event } from "@/types";

export interface DateRange {
  start: Date;
  /** Estremo escluso. */
  end: Date;
}

export interface CalendarRendererProps {
  date: Date;
  events: Event[];
  calendars: Calendar[];
  onSelectSlot: (start: Date, end: Date) => void;
  onSelectEvent: (event: Event) => void;
  /** Spostamento o ridimensionamento da drag & drop (solo eventi non ricorrenti). */
  onChangeEventTime: (event: Event, start: Date, end: Date) => void;
  /** Messaggio breve per l'utente. */
  onNotice: (message: string) => void;
  /** Porta alla vista giorno ("+N altri" del mese). */
  onShowDay: (day: Date) => void;
}

/**
 * Confine di sostituzione del renderer (PRD §18): la UI conosce solo questa interfaccia.
 * Un renderer diverso (es. react-big-calendar o altro) si registra in `renderers.ts`
 * senza toccare Calendar Service, Sync Engine o database.
 */
export interface CalendarRenderer {
  view: CalendarView;
  label: string;
  Component: ComponentType<CalendarRendererProps>;
  /** Intervallo di date da caricare per `date`. */
  getRange: (date: Date) => DateRange;
  /** Data raggiunta spostandosi di un periodo avanti (+1) o indietro (-1). */
  navigate: (date: Date, direction: 1 | -1) => Date;
}
