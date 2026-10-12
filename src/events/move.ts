import type { Account, Calendar, ProviderKind } from "@/types";

/**
 * Perché un evento non può passare dal calendario `from` a `to` (ADR 020), o `null` se può.
 * Specchio delle regole di `move_event`, per spiegarle prima del salvataggio.
 */
export function moveBlockReason(
  from: Calendar | undefined,
  to: Calendar | undefined,
  accounts: Account[],
  change: { attendees: number; conferenceUrl: string },
): string | null {
  if (!from || !to || from.id === to.id) return null;
  const provider = (c: Calendar): ProviderKind | undefined => accounts.find((a) => a.id === c.account_id)?.provider;
  if (change.attendees > 0 && provider(from) !== "local") {
    return "Un evento con partecipanti non si sposta da un calendario sincronizzato: per loro risulterebbe cancellato e ricreato da te.";
  }
  if (change.attendees > 0 && provider(to) === "microsoft") {
    return "Un evento con partecipanti non si sposta su Microsoft: Outlook invierebbe gli inviti.";
  }
  if (change.conferenceUrl.trim() !== "" && provider(to) === "google") {
    return "Un evento con link di videoconferenza non si sposta su Google: il link andrebbe perso.";
  }
  return null;
}
