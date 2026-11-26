/**
 * Fasi principali della Luna (ADR 024): J. Meeus, Astronomical Algorithms, 2a ed., cap. 49, con i
 * termini periodici e le correzioni planetarie. Il risultato di Meeus è in Tempo Dinamico: si
 * sottrae ΔT (polinomi di Espenak e Meeus) per ottenere UTC. Intervallo dichiarato 1900-2100,
 * tolleranza di 2 minuti rispetto alle tabelle USNO.
 */

export type MoonPhase = "new" | "firstQuarter" | "full" | "lastQuarter";

export interface MoonPhaseEvent {
  phase: MoonPhase;
  /** Istante UTC della fase. */
  instant: Date;
}

const PHASES: MoonPhase[] = ["new", "firstQuarter", "full", "lastQuarter"];
const rad = (deg: number) => (deg * Math.PI) / 180;
const sin = (deg: number) => Math.sin(rad(deg));
const cos = (deg: number) => Math.cos(rad(deg));

/** ΔT = TD - UT in secondi (Espenak e Meeus, NASA Five Millennium Canon), per l'anno decimale `y`. */
export function deltaT(y: number): number {
  if (y < 1920) {
    const t = y - 1900;
    return -2.79 + 1.494119 * t - 0.0598939 * t ** 2 + 0.0061966 * t ** 3 - 0.000197 * t ** 4;
  }
  if (y < 1941) {
    const t = y - 1920;
    return 21.2 + 0.84493 * t - 0.0761 * t ** 2 + 0.0020936 * t ** 3;
  }
  if (y < 1961) {
    const t = y - 1950;
    return 29.07 + 0.407 * t - t ** 2 / 233 + t ** 3 / 2547;
  }
  if (y < 1986) {
    const t = y - 1975;
    return 45.45 + 1.067 * t - t ** 2 / 260 - t ** 3 / 718;
  }
  if (y < 2005) {
    const t = y - 2000;
    return 63.86 + 0.3345 * t - 0.060374 * t ** 2 + 0.0017275 * t ** 3 + 0.000651814 * t ** 4 + 0.00002373599 * t ** 5;
  }
  if (y < 2050) {
    const t = y - 2000;
    return 62.92 + 0.32217 * t + 0.005589 * t ** 2;
  }
  const u = (y - 1820) / 100;
  return -20 + 32 * u ** 2 - 0.5628 * (2150 - y);
}

/** JDE (Tempo Dinamico) della fase per il numero di lunazione `k` (intero = luna nuova, +0.25, +0.5, +0.75). */
export function phaseJde(k: number): number {
  const T = k / 1236.85;
  let jde = 2451550.09766 + 29.530588861 * k + 0.00015437 * T ** 2 - 0.00000015 * T ** 3 + 0.00000000073 * T ** 4;
  const E = 1 - 0.002516 * T - 0.0000074 * T ** 2;
  const M = 2.5534 + 29.1053567 * k - 0.0000014 * T ** 2 - 0.00000011 * T ** 3;
  const Mp = 201.5643 + 385.81693528 * k + 0.0107582 * T ** 2 + 0.00001238 * T ** 3 - 0.000000058 * T ** 4;
  const F = 160.7108 + 390.67050284 * k - 0.0016118 * T ** 2 - 0.00000227 * T ** 3 + 0.000000011 * T ** 4;
  const Om = 124.7746 - 1.56375588 * k + 0.0020672 * T ** 2 + 0.00000215 * T ** 3;
  const frac = Math.round((k - Math.floor(k)) * 4) % 4;

  let c: number;
  if (frac === 0 || frac === 2) {
    const full = frac === 2;
    c =
      (full ? -0.40614 : -0.4072) * sin(Mp) +
      (full ? 0.17302 : 0.17241) * E * sin(M) +
      (full ? 0.01614 : 0.01608) * sin(2 * Mp) +
      (full ? 0.01043 : 0.01039) * sin(2 * F) +
      (full ? 0.00734 : 0.00739) * E * sin(Mp - M) -
      (full ? 0.00515 : 0.00514) * E * sin(Mp + M) +
      (full ? 0.00209 : 0.00208) * E * E * sin(2 * M) -
      0.00111 * sin(Mp - 2 * F) -
      0.00057 * sin(Mp + 2 * F) +
      0.00056 * E * sin(2 * Mp + M) -
      0.00042 * sin(3 * Mp) +
      0.00042 * E * sin(M + 2 * F) +
      0.00038 * E * sin(M - 2 * F) -
      0.00024 * E * sin(2 * Mp - M) -
      0.00017 * sin(Om) -
      0.00007 * sin(Mp + 2 * M) +
      0.00004 * sin(2 * Mp - 2 * F) +
      0.00004 * sin(3 * M) +
      0.00003 * sin(Mp + M - 2 * F) +
      0.00003 * sin(2 * Mp + 2 * F) -
      0.00003 * sin(Mp + M + 2 * F) +
      0.00003 * sin(Mp - M + 2 * F) -
      0.00002 * sin(Mp - M - 2 * F) -
      0.00002 * sin(3 * Mp + M) +
      0.00002 * sin(4 * Mp);
  } else {
    c =
      -0.62801 * sin(Mp) +
      0.17172 * E * sin(M) -
      0.01183 * E * sin(Mp + M) +
      0.00862 * sin(2 * Mp) +
      0.00804 * sin(2 * F) +
      0.00454 * E * sin(Mp - M) +
      0.00204 * E * E * sin(2 * M) -
      0.0018 * sin(Mp - 2 * F) -
      0.0007 * sin(Mp + 2 * F) -
      0.0004 * sin(3 * Mp) -
      0.00034 * E * sin(2 * Mp - M) +
      0.00032 * E * sin(M + 2 * F) +
      0.00032 * E * sin(M - 2 * F) -
      0.00028 * E * E * sin(Mp + 2 * M) +
      0.00027 * E * sin(2 * Mp + M) -
      0.00017 * sin(Om) -
      0.00005 * sin(Mp - M - 2 * F) +
      0.00004 * sin(2 * Mp + 2 * F) -
      0.00004 * sin(Mp + M + 2 * F) +
      0.00004 * sin(Mp - 2 * M) +
      0.00003 * sin(Mp + M - 2 * F) +
      0.00003 * sin(3 * M) +
      0.00002 * sin(2 * Mp - 2 * F) +
      0.00002 * sin(Mp - M + 2 * F) -
      0.00002 * sin(3 * Mp + M);
    const W = 0.00306 - 0.00038 * E * cos(M) + 0.00026 * cos(Mp) - 0.00002 * cos(Mp - M) + 0.00002 * cos(Mp + M) + 0.00002 * cos(2 * F);
    c += frac === 1 ? W : -W;
  }

  const A = [
    [299.77 + 0.107408 * k - 0.009173 * T ** 2, 0.000325],
    [251.88 + 0.016321 * k, 0.000165],
    [251.83 + 26.651886 * k, 0.000164],
    [349.42 + 36.412478 * k, 0.000126],
    [84.66 + 18.206239 * k, 0.00011],
    [141.74 + 53.303771 * k, 0.000062],
    [207.14 + 2.453732 * k, 0.00006],
    [154.84 + 7.30686 * k, 0.000056],
    [34.52 + 27.261239 * k, 0.000047],
    [207.19 + 0.121824 * k, 0.000042],
    [291.34 + 1.844379 * k, 0.00004],
    [161.72 + 24.198154 * k, 0.000037],
    [239.56 + 25.513099 * k, 0.000035],
    [331.55 + 3.592518 * k, 0.000023],
  ];
  for (const [arg, coeff] of A) c += coeff * sin(arg);
  jde += c;
  return jde;
}

const JD_UNIX_EPOCH = 2440587.5;

/** Istante UTC della fase `k`: JDE meno ΔT. */
export function phaseInstant(k: number): Date {
  const jde = phaseJde(k);
  const year = 2000 + k / 12.3685;
  const ms = (jde - JD_UNIX_EPOCH) * 86_400_000 - deltaT(year) * 1000;
  return new Date(Math.round(ms));
}

/** Fasi principali con istante in `[start, end)`, in ordine. */
export function moonPhases(start: Date, end: Date): MoonPhaseEvent[] {
  const yearOf = (d: Date) => 2000 + (d.getTime() - Date.UTC(2000, 0, 6)) / (365.25 * 86_400_000);
  // Si parte una lunazione prima, a quarti, e si filtra sul range.
  let k = Math.floor((yearOf(start) - 2000) * 12.3685) - 1;
  const out: MoonPhaseEvent[] = [];
  for (; ; k++) {
    for (let q = 0; q < 4; q++) {
      const instant = phaseInstant(k + q / 4);
      if (instant >= end) return out;
      if (instant >= start) out.push({ phase: PHASES[q], instant });
    }
  }
}
