import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import { calendarColor } from "@/calendar/colors";
import { pickCalendar } from "@/events/draft";
import { dateFormat, errorText, useT } from "@/i18n";
import { useCalendars, useCreateEvent, usePreferences } from "@/providers/queries";
import { useEditorStore } from "@/stores/editorStore";
import { useUiStore } from "@/stores/uiStore";
import { formatTime, localTimezone, toIsoWithOffset } from "@/utils/date";
import { parseQuickAdd } from "@/utils/quickAdd";

const dayFormat = () => dateFormat({ weekday: "long", day: "numeric", month: "long", year: "numeric" });

/** Quick Add `Ctrl + N` (PRD §8, ADR 012): testo libero it/en, anteprima, Invio per creare. */
export function QuickAdd() {
  const t = useT();
  const { quickAddOpen, setQuickAddOpen, setCurrentDate, setNotice } = useUiStore();
  const openNew = useEditorStore((s) => s.openNew);
  const { data: calendars = [] } = useCalendars();
  const create = useCreateEvent();
  const settings = usePreferences();
  const [text, setText] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!quickAddOpen) {
      setText("");
      setError(null);
    }
  }, [quickAddOpen]);

  const parsed = useMemo(() => (text.trim() ? parseQuickAdd(text, new Date(), settings.default_duration) : null), [text, settings.default_duration]);
  const calendar = pickCalendar(calendars, settings.default_calendar_id);

  if (!quickAddOpen) return null;

  const close = () => setQuickAddOpen(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!parsed || !calendar) return;
    if (!parsed.title) return setError(t("events.quickAdd.needTitle"));
    try {
      await create.mutateAsync({
        event: {
          calendar_id: calendar.id,
          title: parsed.title,
          description: null,
          location: null,
          conference_url: null,
          start: toIsoWithOffset(parsed.start),
          end: toIsoWithOffset(parsed.end),
          timezone: localTimezone(),
          all_day: parsed.allDay,
          recurrence_rule: null,
          status: "busy",
          color: null,
          icon: null,
          pattern: null,
        },
        attendees: [],
        reminders: [],
      });
      setCurrentDate(parsed.start);
      setNotice(t("events.quickAdd.created", { title: parsed.title, date: dayFormat().format(parsed.start) }));
      close();
    } catch (err) {
      setError(errorText(err));
    }
  };

  const moreOptions = () => {
    close();
    openNew(parsed ? { start: parsed.start, end: parsed.end } : undefined, { title: parsed?.title ?? text.trim(), allDay: parsed?.allDay ?? false });
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-rail/35 p-4 pt-[18vh]" onClick={(e) => e.target === e.currentTarget && close()}>
      <form role="dialog" aria-label={t("events.quickAdd.dialog")} onSubmit={submit} className="h-fit w-full max-w-lg space-y-3 rounded-lg border bg-popover p-4 text-popover-foreground shadow-[0_18px_48px_rgb(16_20_42/0.28)]">
        <input
          autoFocus
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setError(null);
          }}
          placeholder={t("events.quickAdd.placeholder")}
          className="w-full border-b bg-transparent pb-2 text-[20px] font-semibold outline-none placeholder:font-normal placeholder:text-muted-foreground/70"
        />
        <div aria-live="polite" className="min-h-10 text-sm">
          {parsed && calendar ? (
            <div className="flex items-start gap-2">
              <span className="mt-0.5 h-9 w-1.5 shrink-0 rounded-[1px]" style={{ background: calendarColor(calendars, calendar.id) }} />
              <div>
                <p className="text-[15px] font-bold">{parsed.title || t("events.quickAdd.untitled")}</p>
                <p className="text-muted-foreground">
                  {dayFormat().format(parsed.start)}
                  {parsed.allDay ? ` · ${t("events.quickAdd.allDay")}` : ` · ${formatTime(parsed.start)}–${formatTime(parsed.end)}`} · {calendar.name}
                </p>
              </div>
            </div>
          ) : (
            <p className="text-muted-foreground">
              {!calendar ? t("events.quickAdd.noCalendar") : text.trim() ? t("events.quickAdd.noDate") : t("events.quickAdd.hint")}
            </p>
          )}
          {error && <p role="alert" className="mt-1 text-destructive">{error}</p>}
        </div>
        <div className="flex justify-between">
          <Button type="button" variant="ghost" onClick={moreOptions}>
            {t("events.quickAdd.moreOptions")}
          </Button>
          <Button type="submit" disabled={!parsed || !calendar || create.isPending}>
            {t("events.quickAdd.create")}
          </Button>
        </div>
      </form>
    </div>
  );
}
