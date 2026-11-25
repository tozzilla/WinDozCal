import { describe, expect, it } from "vitest";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { it as itDict } from "./it";

type Tree = Record<string, unknown>;

function flatten(node: unknown, prefix = "", out: Record<string, string> = {}): Record<string, string> {
  for (const [k, v] of Object.entries((node ?? {}) as Tree)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out[key] = v;
    else flatten(v, key, out);
  }
  return out;
}

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");
const pluralBase = (k: string) => k.replace(/_(zero|one|two|few|many|other)$/, "");

const reference = flatten(en);
const known = new Set([...Object.keys(reference), ...Object.keys(flatten(itDict))]);
const knownBases = new Set([...known].map(pluralBase));
const secondary = { es, fr, de } as const;

describe("copertura dizionari secondari (ADR 022)", () => {
  for (const [lang, dict] of Object.entries(secondary)) {
    const flat = flatten(dict);

    it(`${lang}: riepilogo copertura (informativo)`, () => {
      const missing = Object.keys(reference).filter((k) => !(k in flat));
      console.info(`[i18n] ${lang}: ${Object.keys(reference).length - missing.length}/${Object.keys(reference).length} chiavi tradotte, ${missing.length} mancanti`);
      expect(true).toBe(true);
    });

    it(`${lang}: nessuna chiave inesistente in it/en`, () => {
      const unknown = Object.keys(flat).filter((k) => !known.has(k) && !knownBases.has(pluralBase(k)));
      expect(unknown).toEqual([]);
    });

    it(`${lang}: stessi segnaposto dell'inglese`, () => {
      const mismatched = Object.entries(flat)
        .filter(([k, v]) => {
          const ref = reference[k] ?? reference[`${pluralBase(k)}_other`];
          return ref !== undefined && placeholders(v) !== placeholders(ref);
        })
        .map(([k]) => k);
      expect(mismatched).toEqual([]);
    });
  }
});
