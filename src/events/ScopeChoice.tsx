import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { useT } from "@/i18n";
import { scopeLabel, type SeriesScope } from "./seriesEdit";

const SCOPES: SeriesScope[] = ["this", "following", "all"];

/** Scelta "solo questo / questo e i successivi / tutta la serie" (PRD §10). */
export function ScopeChoice({
  question,
  note,
  destructive = false,
  disabled = false,
  scopes = SCOPES,
  onChoose,
  onCancel,
}: {
  question: string;
  note?: string | null;
  destructive?: boolean;
  disabled?: boolean;
  /** Portate offerte (es. solo "Tutta la serie" quando cambia il calendario). */
  scopes?: SeriesScope[];
  onChoose: (scope: SeriesScope) => void;
  onCancel: () => void;
}) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  // Nell'editor la scelta compare in fondo a un form che scorre: la si porta in vista.
  useEffect(() => {
    box.current?.scrollIntoView({ block: "nearest" });
  }, []);
  return (
    <div ref={box} role="group" aria-label={question} className="space-y-2">
      <p className="text-sm font-medium">{question}</p>
      {note && <p className="text-xs text-muted-foreground">{note}</p>}
      <div className="flex flex-wrap gap-2">
        {scopes.map((scope) => (
          <Button key={scope} type="button" variant={destructive ? "destructive" : "default"} disabled={disabled} onClick={() => onChoose(scope)}>
            {scopeLabel(scope)}
          </Button>
        ))}
        <Button type="button" variant="ghost" onClick={onCancel}>
          {t("events.editor.back")}
        </Button>
      </div>
    </div>
  );
}
