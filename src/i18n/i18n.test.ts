import { afterEach, describe, expect, it } from "vitest";
import { errorText, resolveLanguage, setLanguage, t, translate } from "@/i18n";
import { de } from "@/i18n/de";

afterEach(() => setLanguage("it"));

describe("i18n (ADR 022)", () => {
  it("risolve system dalle lingue di sistema, con ripiego sull'inglese", () => {
    expect(resolveLanguage("system", ["de-CH", "en-US"])).toBe("de");
    expect(resolveLanguage("system", ["pt-BR", "fr-FR"])).toBe("fr");
    expect(resolveLanguage("system", ["pt-BR"])).toBe("en");
    expect(resolveLanguage("es", ["it-IT"])).toBe("es");
  });

  it("interpola i parametri nella lingua attiva", () => {
    expect(t("shell.update.later")).toBe("Più tardi");
    setLanguage("en");
    expect(t("shell.update.available", { version: "1.0", current: "0.9" })).toBe("WinDozCal 1.0 is available (installed 0.9).");
  });

  it("una lingua secondaria senza la chiave mostra l'inglese", () => {
    const update = de.shell!.update!;
    const saved = update.later;
    delete update.later;
    try {
      expect(translate("de", "shell.update.later")).toBe("Later");
    } finally {
      update.later = saved;
    }
  });

  it("traduce gli AppError dal codice e lascia gli Error del frontend", () => {
    expect(errorText({ code: "network", message: "timeout" }, "en")).toBe("Network error (timeout)");
    expect(errorText(new Error("Titolo obbligatorio"))).toBe("Titolo obbligatorio");
    expect(errorText({ code: "unknown_code", message: "x" })).toBe("x");
  });
});
