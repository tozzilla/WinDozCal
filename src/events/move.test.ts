import { describe, expect, it } from "vitest";
import type { Account, Calendar, ProviderKind } from "@/types";
import { moveBlockReason } from "./move";

const account = (id: string, provider: ProviderKind): Account => ({ id, provider, name: id, email: "", sync_status: "idle", last_sync: null });
const cal = (id: string, account_id: string): Calendar => ({ id, account_id, remote_id: id, name: id, color: "#000", visible: true, read_only: false });
const accounts = [account("l", "local"), account("g", "google"), account("m", "microsoft")];
const [local, google, outlook] = [cal("pc", "l"), cal("gc", "g"), cal("mc", "m")];
const none = { attendees: 0, conferenceUrl: "" };

describe("spostamento tra calendari (ADR 020)", () => {
  it("senza partecipanti né link si sposta ovunque", () => {
    for (const [from, to] of [[local, google], [google, local], [google, outlook], [outlook, local]]) {
      expect(moveBlockReason(from, to, accounts, none)).toBeNull();
    }
  });
  it("blocca i partecipanti da un calendario sincronizzato o verso Microsoft", () => {
    const withGuests = { attendees: 1, conferenceUrl: "" };
    expect(moveBlockReason(google, local, accounts, withGuests)).toMatch(/sincronizzato/);
    expect(moveBlockReason(local, outlook, accounts, withGuests)).toMatch(/inviti/);
    expect(moveBlockReason(local, google, accounts, withGuests)).toBeNull();
  });
  it("blocca il link di videoconferenza verso Google", () => {
    expect(moveBlockReason(local, google, accounts, { attendees: 0, conferenceUrl: "https://zoom.us/j/1" })).toMatch(/link/);
    expect(moveBlockReason(local, outlook, accounts, { attendees: 0, conferenceUrl: "https://zoom.us/j/1" })).toBeNull();
  });
  it("lo stesso calendario non è uno spostamento", () => {
    expect(moveBlockReason(google, google, accounts, { attendees: 3, conferenceUrl: "x" })).toBeNull();
  });
});
