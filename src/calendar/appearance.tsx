import type { CSSProperties } from "react";
import { t, type TKey } from "@/i18n";
import {
  Briefcase,
  Cake,
  Car,
  Coffee,
  Dumbbell,
  Flag,
  GraduationCap,
  HeartPulse,
  House,
  Music,
  Phone,
  Plane,
  ShoppingCart,
  Users,
  Utensils,
  Video,
  type LucideIcon,
} from "lucide-react";

/** Voce con etichetta tradotta: `label` è un getter, letto al render, così segue il cambio di lingua. */
const colorEntry = (value: string, key: TKey) => ({
  value,
  get label() {
    return t(key);
  },
});

/**
 * Aspetto del singolo evento (ADR 017). I colori sono gli 11 colori evento di Google Calendar (`colorId` 1-11),
 * così la sincronizzazione futura li può mappare senza perdite; le chiavi di icone e pattern coincidono con
 * `EVENT_ICONS` / `EVENT_PATTERNS` del backend.
 */
export const EVENT_COLORS: { value: string; label: string }[] = [
  colorEntry("#7986CB", "calendar.colors.lavender"),
  colorEntry("#33B679", "calendar.colors.sage"),
  colorEntry("#8E24AA", "calendar.colors.grape"),
  colorEntry("#E67C73", "calendar.colors.flamingo"),
  colorEntry("#F6BF26", "calendar.colors.banana"),
  colorEntry("#F4511E", "calendar.colors.tangerine"),
  colorEntry("#039BE5", "calendar.colors.peacock"),
  colorEntry("#616161", "calendar.colors.graphite"),
  colorEntry("#3F51B5", "calendar.colors.blueberry"),
  colorEntry("#0B8043", "calendar.colors.basil"),
  colorEntry("#D50000", "calendar.colors.tomato"),
];

/** Colori proposti per i calendari: quelli dei canali del Palinsesto più la palette degli eventi. */
export const CALENDAR_COLORS: { value: string; label: string }[] = [
  colorEntry("#2F6BFF", "calendar.colors.channelBlue"),
  colorEntry("#1BA672", "calendar.colors.channelGreen"),
  colorEntry("#F2A900", "calendar.colors.channelAmber"),
  ...EVENT_COLORS,
];

const iconEntry = (Icon: LucideIcon, key: TKey) => ({
  Icon,
  get label() {
    return t(key);
  },
});

export const EVENT_ICONS: Record<string, { Icon: LucideIcon; label: string }> = {
  video: iconEntry(Video, "calendar.icons.video"),
  phone: iconEntry(Phone, "calendar.icons.phone"),
  users: iconEntry(Users, "calendar.icons.users"),
  briefcase: iconEntry(Briefcase, "calendar.icons.briefcase"),
  plane: iconEntry(Plane, "calendar.icons.plane"),
  car: iconEntry(Car, "calendar.icons.car"),
  utensils: iconEntry(Utensils, "calendar.icons.utensils"),
  coffee: iconEntry(Coffee, "calendar.icons.coffee"),
  dumbbell: iconEntry(Dumbbell, "calendar.icons.dumbbell"),
  "heart-pulse": iconEntry(HeartPulse, "calendar.icons.heartPulse"),
  cake: iconEntry(Cake, "calendar.icons.cake"),
  "graduation-cap": iconEntry(GraduationCap, "calendar.icons.graduationCap"),
  flag: iconEntry(Flag, "calendar.icons.flag"),
  home: iconEntry(House, "calendar.icons.home"),
  "shopping-cart": iconEntry(ShoppingCart, "calendar.icons.shoppingCart"),
  music: iconEntry(Music, "calendar.icons.music"),
};

/** Pattern di riempimento: disegnati con lo stesso inchiostro tenue del tratteggio del "libero". */
export const EVENT_PATTERNS: Record<string, { label: string; image: string; size?: string }> = {
  dots: {
    get label() {
      return t("calendar.patterns.dots");
    },
    image: "radial-gradient(var(--hatch) 1.2px, transparent 1.4px)",
    size: "7px 7px",
  },
  grid: {
    get label() {
      return t("calendar.patterns.grid");
    },
    image: "linear-gradient(var(--hatch) 1px, transparent 1px), linear-gradient(90deg, var(--hatch) 1px, transparent 1px)",
    size: "8px 8px, 8px 8px",
  },
  lines: {
    get label() {
      return t("calendar.patterns.lines");
    },
    image: "repeating-linear-gradient(0deg, transparent 0 5px, var(--hatch) 5px 6px)",
  },
};

const FREE_HATCH = "repeating-linear-gradient(135deg, transparent 0 5px, var(--hatch) 5px 6px)";

/**
 * Superficie di un evento: banda del calendario sul bordo (il canale resta riconoscibile), tinta del colore
 * dell'evento se scelto, altrimenti del calendario; pattern dell'evento e tratteggio del "libero" sovrapposti.
 */
export function eventSurface(calendarColor: string, free: boolean, appearance?: { color: string | null; pattern: string | null }): CSSProperties {
  const fill = appearance?.color ?? calendarColor;
  const pattern = appearance?.pattern ? EVENT_PATTERNS[appearance.pattern] : undefined;
  const layers = [pattern?.image, free ? FREE_HATCH : undefined].filter(Boolean);
  const sizes = [pattern ? (pattern.size ?? "auto") : undefined, free ? "auto" : undefined].filter(Boolean);
  return {
    backgroundColor: `color-mix(in srgb, ${fill} var(--event-tint), var(--event-mix))`,
    backgroundImage: layers.length ? layers.join(", ") : undefined,
    backgroundSize: sizes.length ? sizes.join(", ") : undefined,
    borderLeft: `5px solid ${calendarColor}`,
  };
}

/** Icona dell'evento, se scelta; dimensione e colore li decide il chiamante. */
export function EventIcon({ name, className }: { name: string | null; className?: string }) {
  const entry = name ? EVENT_ICONS[name] : undefined;
  if (!entry) return null;
  const { Icon, label } = entry;
  return <Icon aria-label={label} className={className} />;
}
