import { openUrl } from "@tauri-apps/plugin-opener";
import { isValidConferenceUrl } from "@/utils/validation";
import { insideTauri } from "./calendarService";

/** Apre un link http/https nel browser di sistema (Tauri) o in una nuova scheda (`npm run dev`). */
export async function openExternal(url: string): Promise<void> {
  if (!isValidConferenceUrl(url)) throw new Error("Il link deve iniziare con http:// o https://");
  if (insideTauri) await openUrl(url.trim());
  else window.open(url.trim(), "_blank", "noopener,noreferrer");
}
