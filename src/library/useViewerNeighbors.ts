import { useEffect, useState } from 'react';
import type { AssetNeighbors, AssetQuery, AssetSummary } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
export function useViewerNeighbors(id: string, query: AssetQuery) {
  const [attempt, setAttempt] = useState(0);
  const [neighbors, setNeighbors] = useState<{
    id: string;
    data?: AssetNeighbors;
    error?: string;
  } | null>(null);
  const serializedQuery = JSON.stringify(query);
  useEffect(() => {
    let active = true;
    const query: AssetQuery = JSON.parse(serializedQuery);
    libraryData({ kind: 'neighbors', query, asset_id: id }, 'neighbors')
      .then((data) => {
        if (active) setNeighbors({ id: id, data });
      })
      .catch((error) => {
        if (active) setNeighbors({ id: id, error: errorMessage(error) });
      });
    return () => {
      active = false;
    };
  }, [serializedQuery, id, attempt]);
  const context = neighbors?.id === id ? neighbors : null;
  return { context, retry: () => setAttempt((value) => value + 1) };
}
export function nearbyPhotos(
  page: AssetSummary[],
  asset: AssetSummary,
  neighbors?: AssetNeighbors,
) {
  const index = page.findIndex((item) => item.id === asset.id);
  if (index >= 0) return page.slice(Math.max(0, index - 6), index + 7);
  return [neighbors?.previous, asset, neighbors?.next].filter((item): item is AssetSummary =>
    Boolean(item),
  );
}
