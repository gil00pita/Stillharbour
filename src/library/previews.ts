import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  THUMBNAIL_POLICY,
  RENDITION_EDGES,
  type AssetSummary,
  type ThumbnailKey,
  type ThumbnailResult,
} from '../bindings';
import { errorMessage, libraryData, nextThumbnailGeneration } from '../ipc';

export function previewKey(key: ThumbnailKey): string {
  return `${key.assetId}:${key.revision}:${key.edge}`;
}
export function previewUrl(asset: AssetSummary, edge: number): string {
  return `${convertFileSrc('', 'thumbnail')}${asset.id}/${asset.revision}/${edge}/${THUMBNAIL_POLICY}`;
}

export function previewEdges(
  results: readonly (ThumbnailResult | undefined)[],
  failed: ReadonlySet<number> = new Set(),
) {
  const candidates = new Set<number>();
  for (const item of results) {
    if (item?.state === 'ready') candidates.add(item.edge);
    if (item?.fallbackEdge !== undefined) candidates.add(item.fallbackEdge);
  }
  return [...candidates]
    .filter((edge) => !failed.has(edge) && RENDITION_EDGES.some((allowed) => allowed === edge))
    .sort((a, b) => b - a);
}

export function usePreviews(
  keys: ThumbnailKey[],
  onError: (error: unknown) => void,
  enabled = true,
  cacheOnlyAbove?: number,
) {
  const [results, setResults] = useState<Record<string, ThumbnailResult>>({});
  const [failure, setFailure] = useState<{ key: string; message: string } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const serialized = JSON.stringify(keys.slice(0, 480));
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const generation = nextThumbnailGeneration();
    const visible: ThumbnailKey[] = JSON.parse(serialized) as ThumbnailKey[];
    async function load() {
      try {
        const groups =
          cacheOnlyAbove === undefined
            ? [{ items: visible, cached: false }]
            : [
                { items: visible.filter((key) => key.edge <= cacheOnlyAbove), cached: false },
                { items: visible.filter((key) => key.edge > cacheOnlyAbove), cached: true },
              ];
        const chunks = groups.flatMap(({ items, cached }) =>
          Array.from({ length: Math.ceil(items.length / 64) }, (_, index) => ({
            keys: items.slice(index * 64, (index + 1) * 64),
            cached,
          })),
        );
        if (!chunks.length) chunks.push({ keys: [], cached: false });
        const batches = await Promise.all(
          chunks.map(({ keys, cached }) =>
            libraryData(
              { kind: 'thumbnails', keys, generation, ...(cached ? { cache_only: true } : {}) },
              'thumbnails',
            ),
          ),
        );
        const response = batches.flat();
        if (!active) return;
        setResults(Object.fromEntries(response.map((item) => [previewKey(item), item])));
        setFailure(null);
        if (response.some((item) => item.state === 'pending'))
          timer = setTimeout(() => void load(), 200);
      } catch (error) {
        if (active) {
          setFailure({ key: serialized, message: errorMessage(error) });
          onError(error);
        }
      }
    }
    void load();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [serialized, onError, enabled, attempt, cacheOnlyAbove]);
  return {
    results,
    attempt,
    error: failure?.key === serialized ? failure.message : null,
    retry: () => {
      setResults({});
      setFailure(null);
      setAttempt((value) => value + 1);
    },
  };
}
