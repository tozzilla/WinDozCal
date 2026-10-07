import type { CSSProperties } from "react";
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

/**
 * Aspetto del singolo evento (ADR 017). I colori sono gli 11 colori evento di Google Calendar (`colorId` 1-11),
 * così la sincronizzazione futura li può mappare senza perdite; le chiavi di icone e pattern coincidono con
 * `EVENT_ICONS` / `EVENT_PATTERNS` del backend.
 */
export const EVENT_COLORS: { value: string; label: string }[] = [
  { value: "#7986CB", label: "Lavanda" },
  { value: "#33B679", label: "Salvia" },
  { value: "#8E24AA", label: "Uva" },
  { value: "#E67C73", label: "Fenicottero" },
  { value: "#F6BF26", label: "Banana" },
  { value: "#F4511E", label: "Mandarino" },
  { value: "#039BE5", label: "Pavone" },
  { value: "#616161", label: "Grafite" },
  { value: "#3F51B5", label: "Mirtillo" },
  { value: "#0B8043", label: "Basilico" },
  { value: "#D50000", label: "Pomodoro" },
];

/** Colori proposti per i calendari: quelli dei canali del Palinsesto più la palette degli eventi. */
export const CALENDAR_COLORS: { value: string; label: string }[] = [
  { value: "#2F6BFF", label: "Blu canale" },
  { value: "#1BA672", label: "Verde canale" },
  { value: "#F2A900", label: "Ambra canale" },
  ...EVENT_COLORS,
];

export const EVENT_ICONS: Record<string, { Icon: LucideIcon; label: string }> = {
  video: { Icon: Video, label: "Videochiamata" },
  phone: { Icon: Phone, label: "Telefonata" },
  users: { Icon: Users, label: "Riunione" },
  briefcase: { Icon: Briefcase, label: "Lavoro" },
  plane: { Icon: Plane, label: "Viaggio" },
  car: { Icon: Car, label: "Spostamento" },
  utensils: { Icon: Utensils, label: "Pasto" },
  coffee: { Icon: Coffee, label: "Pausa" },
  dumbbell: { Icon: Dumbbell, label: "Sport" },
  "heart-pulse": { Icon: HeartPulse, label: "Salute" },
  cake: { Icon: Cake, label: "Compleanno" },
  "graduation-cap": { Icon: GraduationCap, label: "Studio" },
  flag: { Icon: Flag, label: "Scadenza" },
  home: { Icon: House, label: "Casa" },
  "shopping-cart": { Icon: ShoppingCart, label: "Spesa" },
  music: { Icon: Music, label: "Musica" },
};

/** Pattern di riempimento: disegnati con lo stesso inchiostro tenue del tratteggio del "libero". */
export const EVENT_PATTERNS: Record<string, { label: string; image: string; size?: string }> = {
  dots: { label: "Puntini", image: "radial-gradient(var(--hatch) 1.2px, transparent 1.4px)", size: "7px 7px" },
  grid: {
    label: "Quadretti",
    image: "linear-gradient(var(--hatch) 1px, transparent 1px), linear-gradient(90deg, var(--hatch) 1px, transparent 1px)",
    size: "8px 8px, 8px 8px",
  },
  lines: { label: "Righe", image: "repeating-linear-gradient(0deg, transparent 0 5px, var(--hatch) 5px 6px)" },
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
