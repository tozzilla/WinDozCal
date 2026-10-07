import { describe, expect, it } from "vitest";
import { parseQuickAdd } from "./quickAdd";

// Mercoledì 7 ottobre 2026, 21:00 ora locale.
const ref = new Date(2026, 9, 7, 21, 0);
const local = (d: Date) => [d.getFullYear(), d.getMonth() + 1, d.getDate(), d.getHours(), d.getMinutes()];

describe("parseQuickAdd", () => {
  it("riconosce italiano con data relativa e ora (esempio PRD §8)", () => {
    const r = parseQuickAdd("Riunione con il team domani alle 15", ref)!;
    expect(r.title).toBe("Riunione con il team");
    expect(local(r.start)).toEqual([2026, 10, 8, 15, 0]);
    expect(local(r.end)).toEqual([2026, 10, 8, 16, 0]);
    expect(r.allDay).toBe(false);
  });

  it("preferisce la lingua che riconosce più testo", () => {
    const r = parseQuickAdd("Dentista venerdì 9:30", ref)!;
    expect(r.title).toBe("Dentista");
    expect(local(r.start)).toEqual([2026, 10, 9, 9, 30]);
  });

  it("riconosce l'inglese e un intervallo con ora di fine", () => {
    const r = parseQuickAdd("Review next monday 10am to 11am", ref)!;
    expect(r.title).toBe("Review");
    expect(local(r.start)).toEqual([2026, 10, 12, 10, 0]);
    expect(local(r.end)).toEqual([2026, 10, 12, 11, 0]);
  });

  it("senza orario crea un evento di tutto il giorno", () => {
    const r = parseQuickAdd("Compleanno di Anna 3 novembre", ref)!;
    expect(r.title).toBe("Compleanno di Anna");
    expect(r.allDay).toBe(true);
    expect(local(r.start)).toEqual([2026, 11, 3, 0, 0]);
    expect(local(r.end)).toEqual([2026, 11, 4, 0, 0]);
  });

  it("senza data restituisce null", () => {
    expect(parseQuickAdd("Comprare il latte", ref)).toBeNull();
  });
});
