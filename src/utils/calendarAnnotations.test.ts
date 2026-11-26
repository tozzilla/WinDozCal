import { describe, expect, it } from "vitest";
import { buildAnnotations } from "./annotations";
import { easterSunday, holidaysFor } from "./holidays";
import { isoWeek } from "./isoWeek";
import { deltaT, moonPhases, phaseJde, type MoonPhase } from "./moon";

describe("isoWeek", () => {
  const w = (y: number, m: number, d: number) => isoWeek(new Date(y, m - 1, d));
  it("gestisce la settimana 53 e il confine d'anno", () => {
    expect(w(2026, 12, 28)).toEqual({ weekYear: 2026, week: 53 });
    expect(w(2027, 1, 3)).toEqual({ weekYear: 2026, week: 53 });
    expect(w(2027, 1, 4)).toEqual({ weekYear: 2027, week: 1 });
    expect(w(2025, 12, 29)).toEqual({ weekYear: 2026, week: 1 });
  });
  it("numera le settimane interne e i giorni del cambio d'ora", () => {
    expect(w(2026, 11, 26)).toEqual({ weekYear: 2026, week: 48 });
    expect(w(2026, 3, 29)).toEqual({ weekYear: 2026, week: 13 }); // domenica, ora legale
    expect(w(2026, 10, 25)).toEqual({ weekYear: 2026, week: 43 }); // domenica, ora solare
  });
  it("una settimana che parte di domenica attraversa due settimane ISO", () => {
    expect(w(2026, 11, 22).week).toBe(47); // domenica
    expect(w(2026, 11, 23).week).toBe(48); // lunedì
  });
});

describe("easterSunday", () => {
  it.each([
    [1900, 4, 15],
    [2000, 4, 23],
    [2024, 3, 31],
    [2025, 4, 20],
    [2026, 4, 5],
    [2027, 3, 28],
    [2028, 4, 16],
    [2029, 4, 1],
    [2030, 4, 21],
    [2100, 3, 28],
  ])("%i", (y, m, d) => expect(easterSunday(y)).toEqual([m, d]));
});

describe("holidaysFor IT", () => {
  it("2026: tredici festività, con San Francesco dal 2026 (legge 151/2025)", () => {
    expect(holidaysFor("IT", 2026).map((h) => `${h.date} ${h.id}`)).toEqual([
      "2026-01-01 newYear",
      "2026-01-06 epiphany",
      "2026-04-05 easter",
      "2026-04-06 easterMonday",
      "2026-04-25 liberation",
      "2026-05-01 labour",
      "2026-06-02 republic",
      "2026-08-15 assumption",
      "2026-10-04 saintFrancis",
      "2026-11-01 allSaints",
      "2026-12-08 immaculate",
      "2026-12-25 christmas",
      "2026-12-26 saintStephen",
    ]);
  });
  it("2027: Pasquetta il 29 marzo", () => {
    expect(holidaysFor("IT", 2027).find((h) => h.id === "easterMonday")?.date).toBe("2027-03-29");
  });
  it("applica gli anni di validità delle regole", () => {
    const ids = (y: number) => holidaysFor("IT", y).map((h) => h.id);
    expect(ids(2025)).not.toContain("saintFrancis");
    expect(ids(2000)).not.toContain("republic");
    expect(ids(2001)).toContain("republic");
    expect(ids(1985)).not.toContain("epiphany");
  });
});

/** USNO, "Dates of Primary Phases of the Moon" 2026, Universal Time (aa.usno.navy.mil, letto il 26 nov 2026). */
const USNO_2026: [MoonPhase, string][] = [
  ["new", "2026-01-18T19:52"], ["firstQuarter", "2026-01-26T04:47"], ["full", "2026-02-01T22:09"], ["lastQuarter", "2026-02-09T12:43"],
  ["new", "2026-02-17T12:01"], ["firstQuarter", "2026-02-24T12:27"], ["full", "2026-03-03T11:38"], ["lastQuarter", "2026-03-11T09:38"],
  ["new", "2026-03-19T01:23"], ["firstQuarter", "2026-03-25T19:18"], ["full", "2026-04-02T02:12"], ["lastQuarter", "2026-04-10T04:51"],
  ["new", "2026-04-17T11:52"], ["firstQuarter", "2026-04-24T02:32"], ["full", "2026-05-01T17:23"], ["lastQuarter", "2026-05-09T21:10"],
  ["new", "2026-05-16T20:01"], ["firstQuarter", "2026-05-23T11:11"], ["full", "2026-05-31T08:45"], ["lastQuarter", "2026-06-08T10:00"],
  ["new", "2026-06-15T02:54"], ["firstQuarter", "2026-06-21T21:55"], ["full", "2026-06-29T23:56"], ["lastQuarter", "2026-07-07T19:29"],
  ["new", "2026-07-14T09:43"], ["firstQuarter", "2026-07-21T11:05"], ["full", "2026-07-29T14:36"], ["lastQuarter", "2026-08-06T02:21"],
  ["new", "2026-08-12T17:37"], ["firstQuarter", "2026-08-20T02:46"], ["full", "2026-08-28T04:18"], ["lastQuarter", "2026-09-04T07:51"],
  ["new", "2026-09-11T03:27"], ["firstQuarter", "2026-09-18T20:44"], ["full", "2026-09-26T16:49"], ["lastQuarter", "2026-10-03T13:25"],
  ["new", "2026-10-10T15:50"], ["firstQuarter", "2026-10-18T16:12"], ["full", "2026-10-26T04:12"], ["lastQuarter", "2026-11-01T20:28"],
  ["new", "2026-11-09T07:02"], ["firstQuarter", "2026-11-17T11:48"], ["full", "2026-11-24T14:53"], ["lastQuarter", "2026-12-01T06:08"],
  ["new", "2026-12-09T00:52"], ["firstQuarter", "2026-12-17T05:42"], ["full", "2026-12-24T01:28"], ["lastQuarter", "2026-12-30T18:59"],
];

describe("moonPhases", () => {
  it("riproduce l'esempio 49.a di Meeus (luna nuova, febbraio 1977)", () => {
    expect(phaseJde(-283)).toBeCloseTo(2443192.65118, 4);
  });

  it("ΔT nel 2026 è nell'ordine dei 70 secondi", () => {
    expect(deltaT(2026)).toBeGreaterThan(65);
    expect(deltaT(2026)).toBeLessThan(80);
  });

  it("primo quarto a ridosso della mezzanotte UTC (21 dic 2031, ~00:02 UT secondo calendar-12.com)", () => {
    // Caso trovato dalla revisione Codex: con i termini 0.00027·E·sin(2M'+M) e 0.00026·cos M' (Meeus tab. 49)
    // la fase cade dopo la mezzanotte UTC; con i refusi precedenti cadeva il 20 dicembre.
    const [fq] = moonPhases(new Date(Date.UTC(2031, 11, 18)), new Date(Date.UTC(2031, 11, 23)));
    expect(fq.phase).toBe("firstQuarter");
    expect(fq.instant.getTime()).toBeGreaterThanOrEqual(Date.UTC(2031, 11, 21));
    expect(Math.abs(fq.instant.getTime() - Date.UTC(2031, 11, 21, 0, 2))).toBeLessThanOrEqual(2 * 60_000);
  });

  it("tutte le fasi USNO del 2026 entro 2 minuti", () => {
    const got = moonPhases(new Date(Date.UTC(2026, 0, 15)), new Date(Date.UTC(2027, 0, 1)));
    expect(got.map((p) => p.phase)).toEqual(USNO_2026.map(([p]) => p));
    got.forEach((p, i) => {
      const ref = Date.parse(`${USNO_2026[i][1]}:00Z`);
      expect(Math.abs(p.instant.getTime() - ref)).toBeLessThanOrEqual(2 * 60_000);
    });
  });
});

describe("buildAnnotations", () => {
  const utcDay = (offsetMin: number) => (d: Date) => new Date(d.getTime() + offsetMin * 60_000).toISOString().slice(0, 10);

  it("assegna la fase al giorno del fuso indicato (luna piena 29 giugno 2026, 23:56 UT)", () => {
    const range: [Date, Date] = [new Date(Date.UTC(2026, 5, 27)), new Date(Date.UTC(2026, 6, 3))];
    const inUtc = buildAnnotations(...range, { holidayCountry: null, moonPhases: true, dayOf: utcDay(0) });
    const inRome = buildAnnotations(...range, { holidayCountry: null, moonPhases: true, dayOf: utcDay(120) });
    expect(inUtc.get("2026-06-29")?.moon?.phase).toBe("full");
    expect(inRome.get("2026-06-30")?.moon?.phase).toBe("full");
    expect(inRome.has("2026-06-29")).toBe(false);
  });

  it("copre range a cavallo d'anno ed esclude l'estremo finale", () => {
    const a = buildAnnotations(new Date(2026, 11, 28), new Date(2027, 0, 6), { holidayCountry: "IT", moonPhases: false });
    expect(a.get("2027-01-01")?.holidays).toEqual(["newYear"]);
    expect(a.has("2027-01-06")).toBe(false);
    expect(a.has("2026-12-26")).toBe(false);
  });

  it("un paese non supportato non produce festività", () => {
    expect(buildAnnotations(new Date(2026, 0, 1), new Date(2026, 11, 31), { holidayCountry: "XX", moonPhases: false }).size).toBe(0);
  });
});

describe("headerWeek", () => {
  const today = new Date(2026, 10, 26); // giovedì, settimana 48
  it("Settimana usa il lunedì della settimana mostrata, anche con inizio di domenica", async () => {
    const { headerWeek } = await import("@/components/Header");
    expect(headerWeek("week", new Date(2026, 10, 29), today, 1)).toBe(48); // domenica, settimana da lunedì
    expect(headerWeek("week", new Date(2026, 10, 22), today, 0)).toBe(48); // domenica che apre la riga
    expect(headerWeek("day", new Date(2026, 10, 22), today, 0)).toBe(47);
    expect(headerWeek("month", new Date(2026, 10, 1), today, 1)).toBe(48);
    expect(headerWeek("month", new Date(2026, 11, 1), today, 1)).toBeNull();
  });
});
