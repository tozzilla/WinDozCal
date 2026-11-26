import { useEffect, useState } from "react";
import { isSameDay } from "@/utils/date";

/** Data di oggi che cambia a mezzanotte e alla ripresa della finestra (tray, sospensione, cambio di fuso). */
export function useToday(): Date {
  const [today, setToday] = useState(() => new Date());
  useEffect(() => {
    const refresh = () => setToday((prev) => (isSameDay(prev, new Date()) ? prev : new Date()));
    const midnight = () => {
      const now = new Date();
      return new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() - now.getTime() + 1000;
    };
    let timer = window.setTimeout(function tick() {
      refresh();
      timer = window.setTimeout(tick, midnight());
    }, midnight());
    window.addEventListener("focus", refresh);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      window.clearTimeout(timer);
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, []);
  return today;
}
