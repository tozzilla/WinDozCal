import { useState } from "react";
import { CalendarColorPicker } from "@/components/CalendarColorPicker";
import { pickCalendar } from "@/events/draft";
import { useAccounts, useCalendars, useSetCalendarColor, useSetCalendarVisibility, useSettings, useUpdateSettings } from "@/providers/queries";

/** Sezione Calendars (PRD §34): colore, visibilità e calendario predefinito dei nuovi eventi, raggruppati per account. */
export function CalendarsSettings() {
  const { data: accounts = [] } = useAccounts();
  const { data: calendars = [] } = useCalendars();
  const { data: settings } = useSettings();
  const updateSettings = useUpdateSettings();
  // Il predefinito effettivo: quello scelto, oppure quello che l'app userebbe comunque.
  const defaultId = pickCalendar(calendars, settings?.default_calendar_id ?? null)?.id;
  const setColor = useSetCalendarColor();
  const setVisibility = useSetCalendarVisibility();
  const [open, setOpen] = useState<string | null>(null);

  return (
    <div className="max-w-2xl space-y-6">
      {updateSettings.isError && (
        <p role="alert" className="text-sm text-destructive">
          Calendario predefinito non salvato.
        </p>
      )}
      {accounts.map((a) => (
        <section key={a.id}>
          <h3 className="mb-2 text-[11px] font-bold tracking-[0.12em] text-muted-foreground uppercase">{a.name}</h3>
          <ul className="divide-y rounded-lg border bg-card">
            {calendars
              .filter((c) => c.account_id === a.id)
              .map((c) => (
                <li key={c.id} className="p-3">
                  <div className="flex items-center gap-3">
                    <button
                      type="button"
                      aria-expanded={open === c.id}
                      aria-label={`Cambia colore di ${c.name}`}
                      onClick={() => setOpen(open === c.id ? null : c.id)}
                      className="size-6 shrink-0 rounded-full outline-none focus-visible:ring-2 focus-visible:ring-ring"
                      style={{ background: c.color }}
                    />
                    <span className="flex-1 truncate text-[15px] font-semibold">{c.name}</span>
                    {c.read_only ? (
                      <span className="text-[12px] text-muted-foreground">Sola lettura</span>
                    ) : (
                      <label className="flex items-center gap-2 text-[13px] text-muted-foreground">
                        <input
                          type="radio"
                          name="default-calendar"
                          checked={c.id === defaultId}
                          disabled={!settings || updateSettings.isPending}
                          onChange={() => settings && updateSettings.mutate({ ...settings, default_calendar_id: c.id })}
                        />
                        Predefinito
                      </label>
                    )}
                    <label className="flex items-center gap-2 text-[13px] text-muted-foreground">
                      <input type="checkbox" checked={c.visible} onChange={(e) => setVisibility.mutate({ calendarId: c.id, visible: e.target.checked })} />
                      Visibile
                    </label>
                  </div>
                  {open === c.id && (
                    <CalendarColorPicker
                      key={c.id}
                      className="mt-3 pl-9"
                      value={c.color}
                      onPick={(color) => {
                        setColor.mutate({ calendarId: c.id, color });
                        setOpen(null);
                      }}
                    />
                  )}
                </li>
              ))}
          </ul>
        </section>
      ))}
    </div>
  );
}
