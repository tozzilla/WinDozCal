import { useState, type PointerEvent as ReactPointerEvent, type RefObject } from "react";
import type { Calendar, Event } from "@/types";
import { isSameDay } from "@/utils/date";
import { eventInterval, eventKey, isRecurring } from "@/utils/events";
import { moveEvent, resizeEvent } from "./drag";

const DRAG_THRESHOLD_PX = 4;

export interface DragPreview {
  key: string;
  mode: "move" | "resize";
  start: Date;
  end: Date;
}

interface Options {
  days: Date[];
  hourHeight: number;
  /** Elemento che contiene le sole colonne dei giorni (serve la larghezza per i giorni spostati). */
  columnsRef: RefObject<HTMLElement | null>;
  calendars: Calendar[];
  onSelectEvent: (event: Event) => void;
  onChangeEventTime: (event: Event, start: Date, end: Date) => void;
  onNotice: (message: string) => void;
}

/**
 * Drag & drop di un evento con il puntatore: sposta (anche tra giorni) o ridimensiona dal bordo inferiore.
 * Un clic senza movimento seleziona l'evento. Le occorrenze ricorrenti e i calendari in sola lettura
 * non si spostano: si mostra un messaggio. Mentre si trascina, `drag` contiene l'anteprima.
 */
export function useEventDrag({ days, hourHeight, columnsRef, calendars, onSelectEvent, onChangeEventTime, onNotice }: Options) {
  const [drag, setDrag] = useState<DragPreview | null>(null);

  const begin = (event: Event, mode: DragPreview["mode"], down: ReactPointerEvent) => {
    if (down.button !== 0) return;
    down.preventDefault();
    const { start: start0, end: end0 } = eventInterval(event);
    const readOnly = calendars.find((c) => c.id === event.calendar_id)?.read_only ?? false;
    const dayIndex = days.findIndex((d) => isSameDay(d, start0));
    const x0 = down.clientX;
    const y0 = down.clientY;
    let moved = false;
    let blocked = false;
    let preview = { start: start0, end: end0 };

    const onMove = (m: PointerEvent) => {
      if (blocked) return;
      const dx = m.clientX - x0;
      const dy = m.clientY - y0;
      if (!moved) {
        if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
        if (isRecurring(event)) {
          blocked = true;
          return onNotice("Modifica la serie dall'editor");
        }
        if (readOnly) {
          blocked = true;
          return onNotice("Questo calendario è in sola lettura");
        }
        moved = true;
      }
      const columnWidth = (columnsRef.current?.getBoundingClientRect().width ?? 0) / days.length || 1;
      let dayDelta = mode === "move" ? Math.round(dx / columnWidth) : 0;
      if (dayIndex >= 0) dayDelta = Math.min(Math.max(dayDelta, -dayIndex), days.length - 1 - dayIndex);
      const deltaMinutes = (dy / hourHeight) * 60;
      preview = mode === "move" ? moveEvent(start0, end0, dayDelta, deltaMinutes) : resizeEvent(start0, end0, deltaMinutes);
      setDrag({ key: eventKey(event), mode, ...preview });
    };

    const finish = (commit: boolean) => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onCancel);
      setDrag(null);
      if (!commit) return;
      if (!moved && !blocked) onSelectEvent(event);
      else if (moved && (preview.start.getTime() !== start0.getTime() || preview.end.getTime() !== end0.getTime())) {
        onChangeEventTime(event, preview.start, preview.end);
      }
    };
    const onUp = () => finish(true);
    const onCancel = () => finish(false);

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onCancel);
  };

  return { drag, begin };
}
