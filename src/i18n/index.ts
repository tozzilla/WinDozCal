/**
 * Localizzazione dell'interfaccia (ADR 022).
 *
 * - `it` definisce le chiavi; `en` deve averle tutte (lo controlla il typecheck); `es`, `fr`, `de` sono
 *   parziali e ripiegano sull'inglese foglia per foglia.
 * - I dizionari sono divisi per namespace (`shell`, `calendar`, `events`, `settings`, `accounts`, `errors`):
 *   ogni file `src/i18n/<lingua>/<namespace>.ts` esporta un oggetto, unito in `src/i18n/<lingua>/index.ts`.
 * - Interpolazione `{name}`; plurali con il suffisso `_one`/`_other` (e `_few`/`_many` dove la lingua li usa):
 *   `t("events.attendees", { count: 3 })` sceglie `attendees_other` con `Intl.PluralRules`.
 * - Questo modulo non chiama il backend: la lingua arriva dalle impostazioni (`useLanguageSync` in App).
 */
import { useSyncExternalStore } from "react";
import { LANGUAGES, type Language, type LanguageSetting } from "@/types";
import { de } from "./de";
import { en } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { it } from "./it";

/** Struttura del dizionario italiano con i testi allargati a `string`. */
type Widen<T> = { [K in keyof T]: T[K] extends string ? string : Widen<T[K]> };
export type Dictionary = Widen<typeof it>;
export type PartialDictionary = { [K in keyof Dictionary]?: DeepPartial<Dictionary[K]> };
type DeepPartial<T> = { [K in keyof T]?: T[K] extends string ? string : DeepPartial<T[K]> };

type Leaves<T, P extends string = ""> = {
  [K in keyof T & string]: T[K] extends string ? `${P}${K}` : Leaves<T[K], `${P}${K}.`>;
}[keyof T & string];
type PluralSuffix = "_zero" | "_one" | "_two" | "_few" | "_many" | "_other";
type PluralBase<K> = K extends `${infer B}_other` ? B : never;
type LeafKey = Leaves<Dictionary>;
/** Chiave di traduzione: una foglia (`events.save`) o la base di un plurale (`events.attendees`). */
export type TKey = Exclude<LeafKey, `${string}${PluralSuffix}`> | PluralBase<LeafKey>;
export type TParams = Record<string, string | number>;

const DICTIONARIES: Record<Language, PartialDictionary> = { it, en, es, fr, de };
/** Lingue con dizionario completo; le altre sono "traduzione della comunità" (ADR 022). */
export const COMPLETE_LANGUAGES: readonly Language[] = ["it", "en"];
const FALLBACK: Language = "en";

/** Nome di ciascuna lingua nella lingua stessa, per il selettore delle impostazioni. */
export const LANGUAGE_NAMES: Record<Language, string> = {
  it: "Italiano",
  en: "English",
  es: "Español",
  fr: "Français",
  de: "Deutsch",
};

/** Locale `Intl` per lingua quando quella di sistema non ha una regione utile. */
const DEFAULT_LOCALES: Record<Language, string> = { it: "it-IT", en: "en-GB", es: "es-ES", fr: "fr-FR", de: "de-DE" };

function systemLanguages(): readonly string[] {
  if (typeof navigator === "undefined") return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

const primary = (tag: string) => tag.toLowerCase().split(/[-_]/)[0];

/** Lingua effettiva: quella scelta, oppure la prima lingua di sistema supportata, altrimenti l'inglese. */
export function resolveLanguage(setting: LanguageSetting, system: readonly string[] = systemLanguages()): Language {
  if (setting !== "system") return setting;
  for (const tag of system) {
    const lang = primary(tag) as Language;
    if (LANGUAGES.includes(lang)) return lang;
  }
  return FALLBACK;
}

let current: Language = resolveLanguage("system");
const listeners = new Set<() => void>();

export function getLanguage(): Language {
  return current;
}

/** Cambia la lingua attiva; i componenti che usano `useT`/`useLanguage` si ri-renderizzano. */
export function setLanguage(lang: Language): void {
  if (lang === current) return;
  current = lang;
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function useLanguage(): Language {
  return useSyncExternalStore(subscribe, getLanguage, getLanguage);
}

/** Locale `Intl` della lingua attiva: mantiene la regione di sistema se la lingua coincide (en-US, de-CH…). */
export function currentLocale(lang: Language = current): string {
  const match = systemLanguages().find((tag) => primary(tag) === lang && tag.includes("-"));
  return match ?? DEFAULT_LOCALES[lang];
}

function lookup(dict: PartialDictionary, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split(".")) {
    if (node === null || typeof node !== "object") return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

/** Testo nella lingua attiva, con ripiego su inglese e poi italiano; la chiave stessa se manca ovunque. */
function resolve(lang: Language, key: string): string | undefined {
  return lookup(DICTIONARIES[lang], key) ?? lookup(DICTIONARIES[FALLBACK], key) ?? lookup(DICTIONARIES.it, key);
}

/** La chiave esiste nel dizionario italiano (fonte delle chiavi). */
export function hasKey(key: string): boolean {
  return lookup(it, key) !== undefined || lookup(it, `${key}_other`) !== undefined;
}

const pluralRules = new Map<string, Intl.PluralRules>();

export function translate(lang: Language, key: TKey | string, params?: TParams): string {
  let text: string | undefined;
  const count = params?.count;
  if (typeof count === "number") {
    const locale = currentLocale(lang);
    let rules = pluralRules.get(locale);
    if (!rules) pluralRules.set(locale, (rules = new Intl.PluralRules(locale)));
    const category = count === 0 ? "zero" : rules.select(count);
    text =
      lookup(DICTIONARIES[lang], `${key}_${category}`) ??
      (count === 0 ? lookup(DICTIONARIES[lang], `${key}_${rules.select(0)}`) : undefined) ??
      lookup(DICTIONARIES[lang], `${key}_other`);
    if (text === undefined && lang !== FALLBACK) return translate(FALLBACK, key, params);
    text ??= lookup(DICTIONARIES.it, `${key}_${new Intl.PluralRules("it-IT").select(count)}`) ?? lookup(DICTIONARIES.it, `${key}_other`);
  }
  text ??= resolve(lang, key);
  if (text === undefined) return key;
  if (!params) return text;
  return text.replace(/\{(\w+)\}/g, (m, name: string) => (name in params ? String(params[name]) : m));
}

/** Traduzione nella lingua attiva, per codice non React (utilità, store). */
export function t(key: TKey, params?: TParams): string {
  return translate(current, key, params);
}

/** Hook per i componenti: restituisce `t` e ri-renderizza al cambio di lingua. */
export function useT(): (key: TKey, params?: TParams) => string {
  const lang = useLanguage();
  return (key, params) => translate(lang, key, params);
}

const formatters = new Map<string, Intl.DateTimeFormat>();

/** `Intl.DateTimeFormat` nella lingua attiva, in cache per locale e opzioni. Va chiamato al render, non a livello di modulo. */
export function dateFormat(options: Intl.DateTimeFormatOptions): Intl.DateTimeFormat {
  const locale = currentLocale();
  const id = `${locale}|${JSON.stringify(options)}`;
  let f = formatters.get(id);
  if (!f) formatters.set(id, (f = new Intl.DateTimeFormat(locale, options)));
  return f;
}

/**
 * Messaggio leggibile di un errore: gli `AppError` del backend (`{ code, message }`) diventano il testo
 * tradotto di `errors.<code>`, con il messaggio tecnico come dettaglio; gli `Error` del frontend restano come sono.
 */
export function errorText(err: unknown, lang: Language = current): string {
  if (err && typeof err === "object" && "code" in err && typeof (err as { code: unknown }).code === "string") {
    const { code, message } = err as { code: string; message?: unknown };
    const key = `errors.${code}`;
    if (hasKey(key)) {
      const text = translate(lang, key);
      return message ? `${text} (${String(message)})` : text;
    }
  }
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) return String((err as { message: unknown }).message);
  return String(err);
}
