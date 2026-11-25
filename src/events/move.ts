import { t } from "@/i18n";
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
    return t("events.move.syncedWithGuests");
  }
  if (change.attendees > 0 && provider(to) === "microsoft") {
    return t("events.move.microsoftWithGuests");
  }
  if (change.attendees > 0 && provider(to) === "caldav") {
    return t("events.move.caldavWithGuests");
  }
  if (change.conferenceUrl.trim() !== "" && provider(to) === "google") {
    return t("events.move.googleWithConference");
  }
  return null;
}
