import { listen } from "@tauri-apps/api/event";
import { insideTauri } from "./calendarService";

export interface TrayHandlers {
  onNewEvent: () => void;
  onOpenEvent: (eventId: string) => void;
}

/**
 * Ascolta gli eventi emessi dal backend per il system tray (docs/CONTRACT.md).
 * Fuori da Tauri (`npm run dev` nel browser) è un no-op. Restituisce la funzione di cleanup.
 */
export function listenTrayEvents({ onNewEvent, onOpenEvent }: TrayHandlers): () => void {
  if (!insideTauri) return () => {};

  let cancelled = false;
  const unlisteners: Array<() => void> = [];
  const register = (promise: Promise<() => void>) =>
    promise.then((unlisten) => (cancelled ? unlisten() : unlisteners.push(unlisten)));

  void register(listen("tray-new-event", () => onNewEvent()));
  void register(listen<{ eventId: string }>("tray-open-event", (e) => onOpenEvent(e.payload.eventId)));

  return () => {
    cancelled = true;
    unlisteners.forEach((fn) => fn());
  };
}
