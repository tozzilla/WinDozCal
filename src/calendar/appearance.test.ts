import { describe, expect, it } from "vitest";
import { eventSurface } from "./appearance";

describe("eventSurface", () => {
  it("la banda resta del calendario, la tinta segue il colore dell'evento", () => {
    const s = eventSurface("#2F6BFF", false, { color: "#33B679", pattern: null });
    expect(s.borderLeft).toBe("5px solid #2F6BFF");
    expect(s.backgroundColor).toContain("#33B679");
    expect(s.backgroundImage).toBeUndefined();
  });

  it("sovrappone il pattern dell'evento al tratteggio del libero, con una dimensione per strato", () => {
    const s = eventSurface("#2F6BFF", true, { color: null, pattern: "grid" });
    expect(String(s.backgroundImage).match(/gradient\(/g)?.length).toBe(3);
    expect(String(s.backgroundSize).split(",").length).toBe(3);
    expect(s.backgroundColor).toContain("#2F6BFF");
  });
});
