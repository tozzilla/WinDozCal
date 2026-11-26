import { COMPLETE_LANGUAGES, LANGUAGE_NAMES, errorText, useT } from "@/i18n";
import { cn } from "@/lib/utils";
import { useSettings, useUpdateSettings } from "@/providers/queries";
import { DEFAULT_DURATIONS, LANGUAGES, type LanguageSetting, type Settings } from "@/types";
import { HOLIDAY_COUNTRIES, isHolidayCountry } from "@/utils/holidays";

const selectClass = "rounded-md border bg-background px-2 py-1.5 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:opacity-50";

/** Orari a passi di 30 minuti, in minuti dalla mezzanotte (limiti in docs/CONTRACT.md). */
const HALF_HOURS = Array.from({ length: 49 }, (_, i) => i * 30);
const hhmm = (m: number) => `${String(Math.floor(m / 60)).padStart(2, "0")}:${String(m % 60).padStart(2, "0")}`;

function FieldRow({ label, description, children }: { label: string; description?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-6 py-3">
      <div>
        <div className="text-sm font-medium">{label}</div>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-2 text-sm">{children}</div>
    </div>
  );
}

interface SwitchRowProps {
  label: string;
  description?: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}

function SwitchRow({ label, description, checked, disabled, onChange }: SwitchRowProps) {
  return (
    <div className={cn("flex items-start justify-between gap-6 py-3", disabled && "opacity-50")}>
      <div>
        <div className="text-sm font-medium">{label}</div>
        {description && <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative mt-0.5 h-5 w-9 shrink-0 rounded-full border transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50",
          checked ? "border-primary bg-primary" : "bg-muted",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 size-3.5 rounded-full bg-background shadow transition-all",
            checked ? "left-[1.1rem]" : "left-0.5",
          )}
        />
      </button>
    </div>
  );
}

/** Sezione General (PRD §34): avvio, tray e preferenze del calendario. Ogni modifica si salva subito; un errore resta visibile. */
export function GeneralSettings() {
  const t = useT();
  const message = (err: unknown) => (err ? errorText(err) : t("settings.general.unknownError"));
  const { data: settings, isPending, error: loadError } = useSettings();
  const update = useUpdateSettings();

  if (isPending) return <p className="text-sm text-muted-foreground">{t("settings.general.loading")}</p>;
  if (!settings) return <p className="text-sm text-destructive">{t("settings.general.loadFailed", { error: message(loadError) })}</p>;

  const durationLabel = (m: number) =>
    m < 60 ? t("settings.general.minutes", { n: m }) : m === 60 ? t("settings.general.oneHour") : t("settings.general.hours", { n: String(m / 60).replace(".", t("settings.general.decimal")) });

  const save = (patch: Partial<Settings>) => update.mutate({ ...settings, ...patch });

  return (
    <div className="max-w-xl">
      <div className="divide-y">
        <SwitchRow
          label={t("settings.general.startOnLogin")}
          checked={settings.start_on_login}
          disabled={update.isPending}
          onChange={(v) => save({ start_on_login: v })}
        />
        <SwitchRow
          label={t("settings.general.startMinimized")}
          description={t("settings.general.startMinimizedHint")}
          checked={settings.start_minimized}
          disabled={update.isPending || !settings.start_on_login}
          onChange={(v) => save({ start_minimized: v })}
        />
        <SwitchRow
          label={t("settings.general.closeToTray")}
          description={t("settings.general.closeToTrayHint")}
          checked={settings.close_to_tray}
          disabled={update.isPending}
          onChange={(v) => save({ close_to_tray: v })}
        />
        <FieldRow label={t("settings.general.language")} description={t("settings.general.languageHint")}>
          <select
            aria-label={t("settings.general.language")}
            className={selectClass}
            disabled={update.isPending}
            value={settings.language}
            onChange={(e) => save({ language: e.target.value as LanguageSetting })}
          >
            <option value="system">{t("settings.general.languageSystem")}</option>
            {LANGUAGES.map((l) => (
              <option key={l} value={l}>
                {COMPLETE_LANGUAGES.includes(l) ? LANGUAGE_NAMES[l] : `${LANGUAGE_NAMES[l]} (${t("settings.general.languageCommunity")})`}
              </option>
            ))}
          </select>
        </FieldRow>
        <FieldRow label={t("settings.general.weekStart")}>
          <select
            aria-label={t("settings.general.weekStart")}
            className={selectClass}
            disabled={update.isPending}
            value={settings.week_start}
            onChange={(e) => save({ week_start: Number(e.target.value) as Settings["week_start"] })}
          >
            <option value={1}>{t("settings.general.monday")}</option>
            <option value={0}>{t("settings.general.sunday")}</option>
          </select>
        </FieldRow>
        <FieldRow label={t("settings.general.workingHours")} description={t("settings.general.workingHoursHint")}>
          <select
            aria-label={t("settings.general.workStart")}
            className={selectClass}
            disabled={update.isPending}
            value={settings.work_start}
            onChange={(e) => save({ work_start: Number(e.target.value) })}
          >
            {HALF_HOURS.filter((m) => m < settings.work_end).map((m) => (
              <option key={m} value={m}>
                {hhmm(m)}
              </option>
            ))}
          </select>
          <span aria-hidden>–</span>
          <select
            aria-label={t("settings.general.workEnd")}
            className={selectClass}
            disabled={update.isPending}
            value={settings.work_end}
            onChange={(e) => save({ work_end: Number(e.target.value) })}
          >
            {HALF_HOURS.filter((m) => m > settings.work_start).map((m) => (
              <option key={m} value={m}>
                {hhmm(m)}
              </option>
            ))}
          </select>
        </FieldRow>
        <FieldRow label={t("settings.general.defaultDuration")} description={t("settings.general.defaultDurationHint")}>
          <select
            aria-label={t("settings.general.defaultDuration")}
            className={selectClass}
            disabled={update.isPending}
            value={settings.default_duration}
            onChange={(e) => save({ default_duration: Number(e.target.value) })}
          >
            {DEFAULT_DURATIONS.map((m) => (
              <option key={m} value={m}>
                {durationLabel(m)}
              </option>
            ))}
          </select>
        </FieldRow>
        <SwitchRow
          label={t("settings.general.showWeekNumbers")}
          description={t("settings.general.showWeekNumbersHint")}
          checked={settings.show_week_numbers}
          disabled={update.isPending}
          onChange={(v) => save({ show_week_numbers: v })}
        />
        <FieldRow label={t("settings.general.holidayCountry")} description={t("settings.general.holidayCountryHint")}>
          <select
            aria-label={t("settings.general.holidayCountry")}
            className={selectClass}
            disabled={update.isPending}
            value={isHolidayCountry(settings.holiday_country) ? settings.holiday_country : ""}
            onChange={(e) => save({ holiday_country: e.target.value || null })}
          >
            <option value="">{t("settings.general.holidayNone")}</option>
            {HOLIDAY_COUNTRIES.map((c) => (
              <option key={c} value={c}>
                {t(`settings.general.countries.${c}`)}
              </option>
            ))}
          </select>
        </FieldRow>
        <SwitchRow
          label={t("settings.general.showMoonPhases")}
          description={t("settings.general.showMoonPhasesHint")}
          checked={settings.show_moon_phases}
          disabled={update.isPending}
          onChange={(v) => save({ show_moon_phases: v })}
        />
      </div>
      {update.isError && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {t("settings.general.saveFailed", { error: message(update.error) })}
        </p>
      )}
    </div>
  );
}
