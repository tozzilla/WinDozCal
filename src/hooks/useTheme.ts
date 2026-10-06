import { useEffect } from "react";
import { useUiStore } from "@/stores/uiStore";

/** Applica la classe `dark` a <html> secondo il tema scelto; `system` segue `prefers-color-scheme`. */
export function useTheme() {
  const theme = useUiStore((s) => s.theme);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => document.documentElement.classList.toggle("dark", theme === "dark" || (theme === "system" && media.matches));
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);
}
