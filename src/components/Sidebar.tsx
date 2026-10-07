import { useState } from "react";
import { Plus, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useAccounts, useCalendars, useCreateCalendar, useSetCalendarVisibility } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";

const PALETTE = ["#2563eb", "#16a34a", "#9333ea", "#ea580c", "#0891b2", "#db2777"];

/** Sidebar collassabile: account -> calendari con checkbox di visibilità e colore (PRD §6). */
export function Sidebar() {
  const open = useUiStore((s) => s.sidebarOpen);
  const setSettingsOpen = useUiStore((s) => s.setSettingsOpen);
  const { data: accounts = [] } = useAccounts();
  const { data: calendars = [] } = useCalendars();
  const setVisibility = useSetCalendarVisibility();
  const createCalendar = useCreateCalendar();
  const [addingTo, setAddingTo] = useState<string | null>(null);
  const [name, setName] = useState("");

  if (!open) return null;

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r">
      <div className="flex-1 space-y-4 overflow-y-auto p-3">
        {accounts.map((account) => {
          const own = calendars.filter((c) => c.account_id === account.id);
          const allVisible = own.length > 0 && own.every((c) => c.visible);
          return (
            <section key={account.id}>
              <label className="flex items-center gap-2 text-sm font-semibold">
                <input
                  type="checkbox"
                  checked={allVisible}
                  onChange={(e) => own.forEach((c) => setVisibility.mutate({ calendarId: c.id, visible: e.target.checked }))}
                />
                <span className="truncate">{account.name}</span>
                {(account.sync_status === "auth_required" || account.sync_status === "error") && (
                  <button
                    type="button"
                    title={account.sync_status === "auth_required" ? "Accesso da rinnovare: apri Impostazioni > Accounts" : "Errore di sincronizzazione"}
                    aria-label={account.sync_status === "auth_required" ? "Accesso da rinnovare" : "Errore di sincronizzazione"}
                    onClick={(e) => {
                      e.preventDefault();
                      setSettingsOpen(true);
                    }}
                    className="size-2 shrink-0 rounded-full bg-destructive"
                  />
                )}
                {account.provider === "local" && (
                  <Button variant="ghost" size="icon-xs" className="ml-auto" aria-label="Aggiungi calendario locale" onClick={() => setAddingTo(account.id)}>
                    <Plus />
                  </Button>
                )}
              </label>
              {addingTo === account.id && (
                <form
                  className="mt-1 pl-6"
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (!name.trim()) return;
                    createCalendar.mutate({ accountId: account.id, name: name.trim(), color: PALETTE[own.length % PALETTE.length] });
                    setName("");
                    setAddingTo(null);
                  }}
                >
                  <input
                    autoFocus
                    value={name}
                    placeholder="Nome del calendario"
                    onChange={(e) => setName(e.target.value)}
                    onBlur={() => setAddingTo(null)}
                    onKeyDown={(e) => e.key === "Escape" && e.stopPropagation()}
                    className="w-full rounded-md border bg-background px-2 py-1 text-sm outline-none"
                  />
                </form>
              )}
              <ul className="mt-1 space-y-0.5 pl-6">
                {own.map((c) => (
                  <li key={c.id}>
                    <label className="flex items-center gap-2 text-sm">
                      <input
                        type="checkbox"
                        checked={c.visible}
                        style={{ accentColor: c.color }}
                        onChange={(e) => setVisibility.mutate({ calendarId: c.id, visible: e.target.checked })}
                      />
                      <span className="truncate">{c.name}</span>
                    </label>
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </div>
      <div className="border-t p-2">
        <Button variant="ghost" size="sm" className="w-full justify-start" onClick={() => setSettingsOpen(true)}>
          <Settings /> Impostazioni
        </Button>
      </div>
    </aside>
  );
}
