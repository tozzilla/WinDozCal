import { describe, expect, it } from "vitest";
import { layoutOverlaps } from "./layout";

const item = (key: string, start: number, end: number) => ({ key, start, end });

describe("layoutOverlaps", () => {
  it("lascia a colonna piena gli eventi che non si sovrappongono, anche se si toccano", () => {
    const r = layoutOverlaps([item("a", 0, 60), item("b", 60, 120)]);
    expect(r.get("a")).toEqual({ column: 0, columns: 1 });
    expect(r.get("b")).toEqual({ column: 0, columns: 1 });
  });

  it("affianca due eventi sovrapposti", () => {
    const r = layoutOverlaps([item("a", 0, 90), item("b", 30, 120)]);
    expect(r.get("a")).toEqual({ column: 0, columns: 2 });
    expect(r.get("b")).toEqual({ column: 1, columns: 2 });
  });

  it("riusa la prima colonna libera in una catena a-b-c", () => {
    const r = layoutOverlaps([item("a", 0, 60), item("b", 30, 120), item("c", 70, 100)]);
    expect(r.get("a")).toEqual({ column: 0, columns: 2 });
    expect(r.get("b")).toEqual({ column: 1, columns: 2 });
    expect(r.get("c")).toEqual({ column: 0, columns: 2 });
  });

  it("tiene separati i gruppi indipendenti", () => {
    const r = layoutOverlaps([item("a", 0, 60), item("b", 0, 60), item("c", 0, 60), item("d", 200, 260)]);
    expect(r.get("c")).toEqual({ column: 2, columns: 3 });
    expect(r.get("d")).toEqual({ column: 0, columns: 1 });
  });
});
