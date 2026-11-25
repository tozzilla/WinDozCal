import { setLanguage } from "@/i18n";

// I test cercano i testi italiani: lingua di sistema e lingua attiva fissate a `it` (ADR 022).
Object.defineProperty(window.navigator, "languages", { value: ["it-IT"], configurable: true });
Object.defineProperty(window.navigator, "language", { value: "it-IT", configurable: true });
setLanguage("it");
