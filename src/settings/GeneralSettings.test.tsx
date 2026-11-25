import { createRoot } from "react-dom/client";
import { act } from "react";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { calendarService } from "@/providers/calendarService";
import { queryClient } from "@/providers/queries";
import { GeneralSettings } from "./GeneralSettings";

(globalThis as unknown as { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const flush = () => act(async () => { await new Promise((r) => setTimeout(r, 0)); });

describe("GeneralSettings", () => {
  it("offre Sistema più cinque lingue e salva la scelta con l'intero oggetto Settings", async () => {
    const spy = vi.spyOn(calendarService, "updateSettings");
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    act(() => root.render(<QueryClientProvider client={queryClient}><GeneralSettings /></QueryClientProvider>));
    // Le impostazioni arrivano in modo asincrono: sotto carico serve più di un tick.
    let select: HTMLSelectElement | null = null;
    for (let i = 0; i < 50 && !select; i++) {
      await flush();
      select = host.querySelector<HTMLSelectElement>("select[aria-label=Lingua]");
    }
    if (!select) throw new Error("selettore della lingua non renderizzato");
    expect([...select.options].map((o) => o.value)).toEqual(["system", "it", "en", "es", "fr", "de"]);
    expect([...select.options].map((o) => o.textContent)).toContain("Español (traduzione della comunità)");
    expect([...select.options].map((o) => o.textContent)).toContain("Italiano");
    const languageSelect = select;
    act(() => {
      languageSelect.value = "en";
      languageSelect.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await flush();
    expect(spy).toHaveBeenCalledWith(expect.objectContaining({ language: "en", week_start: expect.any(Number) }));
    act(() => root.unmount());
    spy.mockRestore();
  });
});
