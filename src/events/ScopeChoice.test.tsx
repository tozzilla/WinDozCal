import { createRoot } from "react-dom/client";
import { act } from "react";
import { describe, expect, it, vi } from "vitest";
import { ScopeChoice } from "./ScopeChoice";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const labels = (host: HTMLElement) => [...host.querySelectorAll("button")].map((b) => b.textContent);

describe("ScopeChoice", () => {
  it("offre le tre portate, o solo quelle indicate quando cambia il calendario", () => {
    Element.prototype.scrollIntoView = vi.fn();
    const host = document.createElement("div");
    const root = createRoot(host);
    act(() => root.render(<ScopeChoice question="Applicare a" onChoose={vi.fn()} onCancel={vi.fn()} />));
    expect(labels(host)).toEqual(["Solo questo evento", "Questo e i successivi", "Tutta la serie", "Indietro"]);
    act(() => root.render(<ScopeChoice question="Applicare a" scopes={["all"]} onChoose={vi.fn()} onCancel={vi.fn()} />));
    expect(labels(host)).toEqual(["Tutta la serie", "Indietro"]);
    act(() => root.unmount());
  });
});
