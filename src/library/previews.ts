import { useEffect, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import {
  THUMBNAIL_POLICY,
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

export function usePreviews(
  keys: ThumbnailKey[],
  onError: (error: unknown) => void,
  enabled = true,
) {
  const [results, setResults] = useState<Record<string, ThumbnailResult>>({});
  const [failure, setFailure] = useState<{ key: string; message: string } | null>(null);
  const [attempt, setAttempt] = useState(0);
  const serialized = JSON.stringify(keys.slice(0, 240));
  useEffect(() => {
    if (!enabled) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const generation = nextThumbnailGeneration();
    const visible: ThumbnailKey[] = JSON.parse(serialized) as ThumbnailKey[];
    async function load() {
      try {
        const chunks = visible.length
          ? Array.from({ length: Math.ceil(visible.length / 64) }, (_, index) =>
              visible.slice(index * 64, (index + 1) * 64),
            )
          : [[]];
        const batches = await Promise.all(
          chunks.map((chunk) =>
            libraryData({ kind: 'thumbnails', keys: chunk, generation }, 'thumbnails'),
          ),
        );
        const response = batches.flat();
        if (!active) return;
        setResults(Object.fromEntries(response.map((item) => [previewKey(item), item])));
        setFailure(null);
        if (response.some((item) => item.state === 'pending'))
          timer = setTimeout(() => void load(), 750);
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
  }, [serialized, onError, enabled, attempt]);
  return {
    results,
    error: failure?.key === serialized ? failure.message : null,
    retry: () => {
      setFailure(null);
      setAttempt((value) => value + 1);
    },
  };
}
