import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { openLogFolder } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";
import type { ThemeMode } from "@/types";
import { GeneralSettings } from "./GeneralSettings";

// Sezioni PRD §34. Funzionanti: General (tray e avvio), Appearance (tema), link ai log in Advanced.
const SECTIONS = ["General", "Accounts", "Calendars", "Notifications", "Appearance", "Advanced", "About"] as const;
type Section = (typeof SECTIONS)[number];

const THEMES: { value: ThemeMode; label: string }[] = [
  { value: "light", label: "Chiaro" },
  { value: "dark", label: "Scuro" },
  { value: "system", label: "Sistema" },
];

export function SettingsPage() {
  const [section, setSection] = useState<Section>("Appearance");
  const { theme, setTheme, setSettingsOpen } = useUiStore();

  return (
    <div className="flex h-full">
      <nav className="w-52 shrink-0 space-y-1 border-r p-3">
        <Button variant="ghost" size="sm" onClick={() => setSettingsOpen(false)}>
          <ArrowLeft /> Calendario
        </Button>
        {SECTIONS.map((s) => (
          <button
            key={s}
            type="button"
            onClick={() => setSection(s)}
            className={cn("block w-full rounded-md px-3 py-1.5 text-left text-sm hover:bg-accent", s === section && "bg-accent font-medium")}
          >
            {s}
          </button>
        ))}
      </nav>
      <main className="flex-1 p-6">
        <h2 className="mb-4 text-xl font-semibold">{section}</h2>
        {section === "General" && <GeneralSettings />}
        {section === "Appearance" && (
          <div className="flex gap-2">
            {THEMES.map((t) => (
              <Button key={t.value} variant={theme === t.value ? "default" : "outline"} onClick={() => setTheme(t.value)}>
                {t.label}
              </Button>
            ))}
          </div>
        )}
        {section === "Advanced" && (
          <Button variant="outline" onClick={() => void openLogFolder()}>
            Apri cartella dei log
          </Button>
        )}
        {section !== "General" && section !== "Appearance" && section !== "Advanced" && (
          <p className="text-sm text-muted-foreground">Sezione non ancora implementata.</p>
        )}
      </main>
    </div>
  );
}
