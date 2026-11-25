import { useEffect } from "react";
import { resolveLanguage, setLanguage } from "@/i18n";
import { calendarService } from "@/providers/calendarService";
import { usePreferences } from "@/providers/queries";

/**
 * Applica la lingua delle impostazioni (ADR 022): risolve `system` con le lingue della WebView, aggiorna
 * l'interfaccia e comunica la lingua effettiva al backend (tray e notifiche).
 */
export function useLanguageSync() {
  const { language } = usePreferences();
  const resolved = resolveLanguage(language);
  useEffect(() => {
    setLanguage(resolved);
    document.documentElement.lang = resolved;
    void calendarService.setUiLanguage(resolved).catch(() => undefined);
  }, [resolved]);
}
