import { useState } from "react";
import { ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useT, type TKey } from "@/i18n";
import { cn } from "@/lib/utils";
import { openLogFolder, useAppVersion, useUpdateCheck } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";
import type { ThemeMode } from "@/types";
import { AccountsSettings } from "./AccountsSettings";
import { CalendarsSettings } from "./CalendarsSettings";
import { GeneralSettings } from "./GeneralSettings";

// Sezioni PRD §34. Funzionanti: General, Accounts, Calendars, Appearance (tema e compact mode), link ai log in Advanced, About.
// Notifications: il PRD non elenca voci (ADR 018).
const SECTIONS = ["General", "Accounts", "Calendars", "Notifications", "Appearance", "Advanced", "About"] as const;
type Section = (typeof SECTIONS)[number];

const THEMES: ThemeMode[] = ["light", "dark", "system"];

export function SettingsPage() {
  const t = useT();
  const sectionLabel = (s: Section) => t(`settings.sections.${s.toLowerCase()}` as TKey);
  const [section, setSection] = useState<Section>("Appearance");
  const { theme, setTheme, compact, setCompact, setSettingsOpen } = useUiStore();

  return (
    <div className="flex h-full">
      <nav className="flex w-62 shrink-0 flex-col gap-0.5 bg-rail p-3 text-rail-foreground">
        <button
          type="button"
          onClick={() => setSettingsOpen(false)}
          className="mb-4 flex items-center gap-2 rounded-sm px-1 py-1.5 text-[14px] font-semibold text-rail-muted outline-none hover:text-rail-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring"
        >
          <ArrowLeft className="size-4" /> {t("settings.back")}
        </button>
        <span className="mb-1 px-1 text-[11px] font-bold tracking-[0.12em] text-rail-foreground/70 uppercase">{t("settings.title")}</span>
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
            {sectionLabel(s)}
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-y-auto bg-background p-8">
        <h2 className="mb-6 text-[26px] font-bold tracking-[-0.01em]">{sectionLabel(section)}</h2>
        {section === "General" && <GeneralSettings />}
        {section === "Accounts" && <AccountsSettings />}
        {section === "Calendars" && <CalendarsSettings />}
        {section === "Appearance" && (
          <div className="max-w-xl space-y-6">
            <div className="flex gap-2">
              {THEMES.map((value) => (
                <Button key={value} variant={theme === value ? "default" : "outline"} onClick={() => setTheme(value)}>
                  {t(`settings.theme.${value}` as TKey)}
                </Button>
              ))}
            </div>
            <label className="flex items-start justify-between gap-6">
              <span>
                <span className="block text-sm font-medium">{t("settings.compact.label")}</span>
                <span className="mt-0.5 block text-xs text-muted-foreground">{t("settings.compact.description")}</span>
              </span>
              <input type="checkbox" className="mt-1 size-4" checked={compact} onChange={(e) => setCompact(e.target.checked)} />
            </label>
          </div>
        )}
        {section === "Advanced" && (
          <Button variant="outline" onClick={() => void openLogFolder()}>
            {t("settings.advanced.openLogs")}
          </Button>
        )}
        {section === "About" && <About />}
        {section !== "General" && section !== "Accounts" && section !== "Calendars" && section !== "Appearance" && section !== "Advanced" && section !== "About" && (
          <p className="text-sm text-muted-foreground">{t("settings.notImplemented")}</p>
        )}
      </main>
    </div>
  );
}

/** Versione installata e controllo manuale degli aggiornamenti (PRD §34, §36). */
function About() {
  const t = useT();
  const { data: version } = useAppVersion();
  const update = useUpdateCheck();
  return (
    <div className="space-y-3 text-sm">
      <p>WinDozCal {version}</p>
      <div className="flex items-center gap-3">
        <Button variant="outline" disabled={update.isFetching} onClick={() => void update.refetch()}>
          {t("settings.about.checkUpdates")}
        </Button>
        <span className="text-muted-foreground">
          {update.isFetching
            ? t("settings.about.checking")
            : update.isError
              ? t("settings.about.checkFailed")
              : update.data
                ? t("settings.about.available", { version: update.data.version })
                : update.isFetched
                  ? t("settings.about.upToDate")
                  : ""}
        </span>
      </div>
    </div>
  );
}
