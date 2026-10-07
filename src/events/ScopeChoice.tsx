import { useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { SCOPE_LABEL, type SeriesScope } from "./seriesEdit";

const SCOPES: SeriesScope[] = ["this", "following", "all"];

/** Scelta "solo questo / questo e i successivi / tutta la serie" (PRD §10). */
export function ScopeChoice({
  question,
  note,
  destructive = false,
  disabled = false,
  onChoose,
  onCancel,
}: {
  question: string;
  note?: string | null;
  destructive?: boolean;
  disabled?: boolean;
  onChoose: (scope: SeriesScope) => void;
  onCancel: () => void;
}) {
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
        {SCOPES.map((scope) => (
          <Button key={scope} type="button" variant={destructive ? "destructive" : "default"} disabled={disabled} onClick={() => onChoose(scope)}>
            {SCOPE_LABEL[scope]}
          </Button>
        ))}
        <Button type="button" variant="ghost" onClick={onCancel}>
          Indietro
        </Button>
      </div>
    </div>
  );
}
