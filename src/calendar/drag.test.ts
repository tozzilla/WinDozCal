import { describe, expect, it } from "vitest";
import { clickRange, moveEvent, resizeEvent, slotRange, snapToStep } from "./drag";

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

  it("crea trascinando su una fascia vuota, anche verso l'alto, agganciato a 15 minuti", () => {
    const day = at(0);
    const down = slotRange(day, 14 * 60 + 7, 15 * 60 + 52); // 14:07 -> 15:52
    expect([down.start.getTime(), down.end.getTime()]).toEqual([at(14, 0).getTime(), at(16, 0).getTime()]);
    const up = slotRange(day, 16 * 60 + 10, 15 * 60 + 20); // trascinato verso l'alto
    expect([up.start.getTime(), up.end.getTime()]).toEqual([at(15, 15).getTime(), at(16, 15).getTime()]);
    const tiny = slotRange(day, 9 * 60 + 2, 9 * 60 + 4);
    expect(tiny.end.getTime() - tiny.start.getTime()).toBe(15 * 60_000);
  });

  it("un clic crea un'ora dall'ora intera cliccata, senza superare la mezzanotte", () => {
    const r = clickRange(at(0), 14 * 60 + 40);
    expect([r.start.getTime(), r.end.getTime()]).toEqual([at(14, 0).getTime(), at(15, 0).getTime()]);
    const late = clickRange(at(0), 23 * 60 + 30);
    expect([late.start.getTime(), late.end.getTime()]).toEqual([at(23, 0).getTime(), at(0, 0, 7).getTime()]);
  });
});
