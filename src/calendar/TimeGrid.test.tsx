import { createRoot } from "react-dom/client";
import { act } from "react";
import { describe, expect, it, vi } from "vitest";
import { WeekView } from "./WeekView";
import { toIsoWithOffset } from "@/utils/date";
import type { Event } from "@/types";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const ev = (id: string, h: number, rule: string | null = null): Event => ({
  id, calendar_id: "c", remote_id: null, title: id, description: null, location: null, conference_url: null,
  start: toIsoWithOffset(new Date(2026, 9, 6, h)), end: toIsoWithOffset(new Date(2026, 9, 6, h + 2)), timezone: "Europe/Rome",
  all_day: false, recurrence_rule: rule, status: "busy", etag: null, updated_at: null, sync_status: "synced",
  local_updated_at: null, remote_updated_at: null, occurrence_start: null, series_id: null, original_start: null,
});

describe("TimeGrid", () => {
  it("rende eventi sovrapposti affiancati e gestisce drag e clic", async () => {
    const onSelectEvent = vi.fn();
    const onChangeEventTime = vi.fn();
    const onNotice = vi.fn();
    const host = document.createElement("div");
    document.body.appendChild(host);
    await act(async () => {
      createRoot(host).render(
        <WeekView
          date={new Date(2026, 9, 6)}
          events={[ev("a", 10), ev("b", 11), ev("r", 15, "RRULE:FREQ=DAILY")]}
          calendars={[]}
          onSelectSlot={() => {}}
          onSelectEvent={onSelectEvent}
          onChangeEventTime={onChangeEventTime}
          onNotice={onNotice}
          onShowDay={() => {}}
        />,
      );
    });
    const nodes = [...host.querySelectorAll<HTMLElement>("[data-event]")];
    expect(nodes.map((n) => n.style.getPropertyValue("--side-width"))).toEqual(["50%", "50%", "100%"]);

    const fire = (type: string, x: number, y: number) =>
      window.dispatchEvent(Object.assign(new Event(type), { clientX: x, clientY: y, button: 0 }));
    const down = (el: HTMLElement, x: number, y: number) =>
      el.dispatchEvent(Object.assign(new Event("pointerdown", { bubbles: true }), { clientX: x, clientY: y, button: 0, preventDefault() {} }));

    // clic senza movimento -> seleziona
    await act(async () => { down(nodes[0], 5, 5); fire("pointerup", 5, 5); });
    expect(onSelectEvent).toHaveBeenCalledTimes(1);

    // drag verticale di 72px (1 ora) -> sposta, snap a 15 minuti
    await act(async () => { down(nodes[0], 5, 5); fire("pointermove", 5, 77); });
    await act(async () => { fire("pointerup", 5, 77); });
    expect(onChangeEventTime).toHaveBeenCalledTimes(1);
    expect(new Date(onChangeEventTime.mock.calls[0][1]).getHours()).toBe(11);

    // occorrenza ricorrente -> si sposta anch'essa: la portata (solo questo / successivi / tutta) la chiede il chiamante
    await act(async () => { down(nodes[2], 5, 5); fire("pointermove", 5, 60); fire("pointerup", 5, 60); });
    expect(onNotice).not.toHaveBeenCalled();
    expect(onChangeEventTime).toHaveBeenCalledTimes(2);
    expect(onChangeEventTime.mock.calls[1][0].id).toBe("r");
  });
});
