import { useState } from "react";
import { Plus, Settings } from "lucide-react";
import { useAccounts, useCalendars, useCreateCalendar, useSetCalendarVisibility } from "@/providers/queries";
import { useUiStore } from "@/stores/uiStore";
import type { Account } from "@/types";

const PALETTE = ["#2F6BFF", "#1BA672", "#F2A900", "#9C6ADE", "#0891B2", "#E85D9A"];

const PROVIDER_LABEL: Record<Account["provider"], string> = {
  local: "",
  google: "Google",
  microsoft: "Microsoft",
  caldav: "CalDAV",
};

/** Intestazione del canale: per gli account esterni "Provider · nome", per quello locale il suo nome. */
function channelName(a: Account): string {
  const provider = PROVIDER_LABEL[a.provider];
  return provider && a.name !== provider ? `${provider} · ${a.name}` : a.name;
}

/**
 * Rail dei canali (PRD §6, Palinsesto): ogni account è un canale con la sua barretta d'identità, i calendari
 * hanno il quadratino colore come casella di visibilità. Collassabile dall'header.
 */
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
    <aside className="flex w-52 shrink-0 flex-col bg-rail text-rail-foreground xl:w-62">
      <div className="px-3.5 pt-3.5 pb-3 text-[17px] font-bold tracking-[0.02em]">WinDozCal</div>
      <div className="flex-1 space-y-[18px] overflow-y-auto px-3 pb-4">
        {accounts.map((account) => {
          const own = calendars.filter((c) => c.account_id === account.id);
          const allVisible = own.length > 0 && own.every((c) => c.visible);
          const ident = own[0]?.color ?? "#9AA1BD";
          const attention = account.sync_status === "auth_required" || account.sync_status === "error";
          return (
            <section key={account.id} aria-label={channelName(account)}>
              <div className="group mb-2 flex items-center gap-2 px-1">
                <button
                  type="button"
                  aria-pressed={allVisible}
                  title={allVisible ? "Nascondi tutto il canale" : "Mostra tutto il canale"}
                  onClick={() => own.forEach((c) => setVisibility.mutate({ calendarId: c.id, visible: !allVisible }))}
                  className="flex min-w-0 flex-1 items-center gap-2 rounded-sm text-left leading-[13px] outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring"
                >
                  <span aria-hidden className="h-1.5 w-5 shrink-0 rounded-[1px]" style={{ background: ident, opacity: allVisible ? 1 : 0.4 }} />
                  <span className="truncate text-[11px] font-bold tracking-[0.12em] text-rail-foreground/70 uppercase">{channelName(account)}</span>
                </button>
                {attention && (
                  <button
                    type="button"
                    title={account.sync_status === "auth_required" ? "Accesso da rinnovare: apri Impostazioni > Accounts" : "Errore di sincronizzazione"}
                    aria-label={account.sync_status === "auth_required" ? "Accesso da rinnovare" : "Errore di sincronizzazione"}
                    onClick={() => setSettingsOpen(true)}
                    className="size-2 shrink-0 rounded-full bg-onair"
                  />
                )}
                {account.provider === "local" && (
                  <button
                    type="button"
                    aria-label="Aggiungi calendario locale"
                    onClick={() => setAddingTo(account.id)}
                    className="-my-1.5 grid size-6 place-items-center rounded-sm text-rail-muted opacity-0 outline-none group-hover:opacity-100 hover:bg-rail-accent hover:text-rail-foreground focus-visible:opacity-100"
                  >
                    <Plus className="size-3.5" />
                  </button>
                )}
              </div>
              {addingTo === account.id && (
                <form
                  className="mt-1 px-1"
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
                    className="w-full rounded-sm border border-sidebar-border bg-rail-accent px-2 py-1 text-sm text-rail-foreground outline-none placeholder:text-rail-muted focus-visible:ring-2 focus-visible:ring-sidebar-ring"
                  />
                </form>
              )}
              <ul className="mt-1 space-y-0.5">
                {own.map((c) => (
                  <li key={c.id}>
                    <button
                      type="button"
                      role="checkbox"
                      aria-checked={c.visible}
                      onClick={() => setVisibility.mutate({ calendarId: c.id, visible: !c.visible })}
                      className="flex w-full items-center gap-2.5 rounded-sm px-1 py-[3px] text-left text-[14px] leading-[21px] font-semibold outline-none hover:bg-rail-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring"
                    >
                      <span
                        aria-hidden
                        className="grid size-3.5 shrink-0 place-items-center rounded-[2px] border-2"
                        style={{ background: c.visible ? c.color : "transparent", borderColor: c.color }}
                      >
                      </span>
                      <span className={c.visible ? "truncate" : "truncate text-rail-muted"}>{c.name}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </div>
      <div className="px-3 pb-3">
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          className="flex w-full items-center gap-2 rounded-sm px-1 py-1.5 text-[14px] font-semibold text-rail-muted outline-none hover:bg-rail-accent hover:text-rail-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring"
        >
          <Settings className="size-4" /> Impostazioni
        </button>
      </div>
    </aside>
  );
}
