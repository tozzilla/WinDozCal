import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { openLogFolder, useAppVersion, useUpdateCheck } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";
import type { ThemeMode } from "@/types";
import { AccountsSettings } from "./AccountsSettings";
import { CalendarsSettings } from "./CalendarsSettings";
import { GeneralSettings } from "./GeneralSettings";

// Sezioni PRD §34. Funzionanti: General (tray e avvio), Accounts, Calendars, Appearance (tema), link ai log in Advanced, About.
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
      <nav className="flex w-62 shrink-0 flex-col gap-0.5 bg-rail p-3 text-rail-foreground">
        <button
          type="button"
          onClick={() => setSettingsOpen(false)}
          className="mb-4 flex items-center gap-2 rounded-sm px-1 py-1.5 text-[14px] font-semibold text-rail-muted outline-none hover:text-rail-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring"
        >
          <ArrowLeft className="size-4" /> Calendario
        </button>
        <span className="mb-1 px-1 text-[11px] font-bold tracking-[0.12em] text-rail-foreground/70 uppercase">Impostazioni</span>
        {SECTIONS.map((s) => (
          <button
            key={s}
            type="button"
            aria-current={s === section ? "page" : undefined}
            onClick={() => setSection(s)}
            className={cn(
              "flex items-center gap-2 rounded-sm px-2 py-1.5 text-left text-[14px] font-semibold outline-none hover:bg-rail-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring",
              s === section ? "bg-rail-accent text-rail-foreground" : "text-rail-foreground/80",
            )}
          >
            <span aria-hidden className={cn("h-4 w-1 rounded-[1px]", s === section ? "bg-onair" : "bg-transparent")} />
            {s}
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-y-auto bg-background p-8">
        <h2 className="mb-6 text-[26px] font-bold tracking-[-0.01em]">{section}</h2>
        {section === "General" && <GeneralSettings />}
        {section === "Accounts" && <AccountsSettings />}
        {section === "Calendars" && <CalendarsSettings />}
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
        {section === "About" && <About />}
        {section !== "General" && section !== "Accounts" && section !== "Calendars" && section !== "Appearance" && section !== "Advanced" && section !== "About" && (
          <p className="text-sm text-muted-foreground">Sezione non ancora implementata.</p>
        )}
      </main>
    </div>
  );
}

/** Versione installata e controllo manuale degli aggiornamenti (PRD §34, §36). */
function About() {
  const { data: version } = useAppVersion();
  const update = useUpdateCheck();
  return (
    <div className="space-y-3 text-sm">
      <p>WinDozCal {version}</p>
      <div className="flex items-center gap-3">
        <Button variant="outline" disabled={update.isFetching} onClick={() => void update.refetch()}>
          Controlla aggiornamenti
        </Button>
        <span className="text-muted-foreground">
          {update.isFetching
            ? "Controllo in corso…"
            : update.isError
              ? "Controllo non riuscito."
              : update.data
                ? `Disponibile la versione ${update.data.version}.`
                : update.isFetched
                  ? "Hai l'ultima versione."
                  : ""}
        </span>
      </div>
    </div>
  );
}
