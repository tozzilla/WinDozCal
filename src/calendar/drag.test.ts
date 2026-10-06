import { describe, expect, it } from "vitest";
import { moveEvent, resizeEvent, snapToStep } from "./drag";

const at = (h: number, m = 0, day = 6) => new Date(2026, 9, day, h, m);

describe("drag", () => {
  it("aggancia a 15 minuti", () => {
    expect(snapToStep(at(10, 7)).getTime()).toBe(at(10, 0).getTime());
    expect(snapToStep(at(10, 8)).getTime()).toBe(at(10, 15).getTime());
  });

  it("sposta tra giorni e orari mantenendo la durata", () => {
    const r = moveEvent(at(10, 0), at(11, 30), 2, 40); // +2 giorni, +40 minuti -> 10:40 -> 10:45
    expect(r.start.getTime()).toBe(at(10, 45, 8).getTime());
    expect(r.end.getTime()).toBe(at(12, 15, 8).getTime());
  });

  it("ridimensiona dal bordo inferiore senza scendere sotto i 15 minuti", () => {
    expect(resizeEvent(at(10, 0), at(11, 0), 31).end.getTime()).toBe(at(11, 30).getTime());
    expect(resizeEvent(at(10, 0), at(11, 0), -120).end.getTime()).toBe(at(10, 15).getTime());
  });
});
