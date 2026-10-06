export interface LayoutInput {
  key: string;
  /** Millisecondi dall'epoca. */
  start: number;
  end: number;
}

export interface LayoutResult {
  /** Colonna (0-based) in cui disegnare l'evento. */
  column: number;
  /** Numero di colonne del gruppo di eventi sovrapposti a cui appartiene. */
  columns: number;
}

/**
 * Affianca gli eventi sovrapposti: ogni gruppo di eventi collegati da sovrapposizioni condivide
 * lo stesso numero di colonne; ogni evento prende la prima colonna libera. Eventi che si toccano
 * (fine = inizio) non si sovrappongono.
 */
export function layoutOverlaps(items: LayoutInput[]): Map<string, LayoutResult> {
  const sorted = [...items].sort((a, b) => a.start - b.start || b.end - a.end || a.key.localeCompare(b.key));
  const result = new Map<string, LayoutResult>();

  let cluster: { key: string; column: number }[] = [];
  let columnEnds: number[] = [];
  let clusterEnd = -Infinity;

  const closeCluster = () => {
    for (const m of cluster) result.set(m.key, { column: m.column, columns: columnEnds.length });
    cluster = [];
    columnEnds = [];
  };

  for (const item of sorted) {
    if (item.start >= clusterEnd) closeCluster();
    let column = columnEnds.findIndex((end) => end <= item.start);
    if (column === -1) column = columnEnds.length;
    columnEnds[column] = item.end;
    cluster.push({ key: item.key, column });
    clusterEnd = Math.max(clusterEnd, item.end);
  }
  closeCluster();
  return result;
}
