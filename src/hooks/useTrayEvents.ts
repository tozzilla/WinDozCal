import { useEffect } from "react";
import { fetchEventDetail } from "@/providers/queries";
import { listenTrayEvents } from "@/providers/trayEvents";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";

/** Reagisce ai comandi del tray: nuovo evento (come Ctrl+N) o apertura di un evento esistente. */
export function useTrayEvents() {
  useEffect(
    () =>
      listenTrayEvents({
        onNewEvent: () => {
          useUiStore.getState().setSettingsOpen(false);
          useEditorStore.getState().openNew();
        },
        onOpenEvent: (eventId) => {
          void fetchEventDetail(eventId)
            .then(({ event }) => {
              const ui = useUiStore.getState();
              ui.setSettingsOpen(false);
              ui.setCurrentDate(new Date(event.start));
              useEditorStore.getState().openExisting(event);
            })
            // Evento cancellato nel frattempo: la finestra resta com'è.
            .catch((err) => console.warn("tray-open-event: evento non disponibile", err));
        },
      }),
    [],
  );
}
