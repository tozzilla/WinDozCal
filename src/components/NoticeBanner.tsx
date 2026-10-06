import { useEffect } from "react";
import { useUiStore } from "@/stores/uiStore";

const VISIBLE_MS = 4000;

/** Messaggio breve in basso, non bloccante; sparisce da solo. */
export function NoticeBanner() {
  const notice = useUiStore((s) => s.notice);
  const setNotice = useUiStore((s) => s.setNotice);

  useEffect(() => {
    if (!notice) return;
    const id = setTimeout(() => setNotice(null), VISIBLE_MS);
    return () => clearTimeout(id);
  }, [notice, setNotice]);

  if (!notice) return null;
  return (
    <div
      role="status"
      className="fixed bottom-4 left-1/2 z-[60] -translate-x-1/2 rounded-lg border bg-popover px-4 py-2 text-sm text-popover-foreground shadow-lg"
    >
      {notice}
    </div>
  );
}
