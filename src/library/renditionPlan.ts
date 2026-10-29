import {
  GALLERY_DECODED_BYTES,
  GALLERY_MAX_EDGE,
  RENDITION_EDGES,
  type AssetSummary,
  type ThumbnailKey,
} from '../bindings';
import type { PhotoRow } from './model';

const galleryEdges = RENDITION_EDGES.filter((edge) => edge >= 256 && edge <= GALLERY_MAX_EDGE);
export function pixelRatio(value: number) {
  return Number.isFinite(value) ? Math.max(1, Math.min(3, value)) : 1;
}
function decodedBytes(asset: AssetSummary, edge: number) {
  const ratio = asset.width && asset.height ? asset.width / asset.height : 1;
  return Math.max(1, Math.ceil(edge * Math.min(ratio, 1 / ratio))) * edge * 4;
}
export function galleryRenditions(rows: PhotoRow[], ratio: number, cover: boolean) {
  const entries = rows
    .flatMap((row) =>
      row.assets.map((asset, index) => ({
        asset,
        width: row.widths[index] ?? row.imageHeight,
        height: row.imageHeight,
      })),
    )
    .slice(0, 240);
  let remaining =
    GALLERY_DECODED_BYTES -
    entries.length * 32 * 32 * 4 -
    entries.reduce((sum, entry) => sum + decodedBytes(entry.asset, 256), 0);
  return new Map(
    entries.map(({ asset, width, height }) => {
      const sourceWidth = asset.width && asset.width > 0 ? asset.width : width;
      const sourceHeight = asset.height && asset.height > 0 ? asset.height : height;
      const scale = cover
        ? Math.max(width / sourceWidth, height / sourceHeight)
        : Math.min(width / sourceWidth, height / sourceHeight);
      const required = Math.max(sourceWidth, sourceHeight) * scale * pixelRatio(ratio);
      let index = galleryEdges.findIndex((edge) => edge >= required);
      if (index < 0) index = galleryEdges.length - 1;
      // This is a decoded-pixel admission estimate, not a WebView/GPU RSS quota.
      while (
        index > 0 &&
        decodedBytes(asset, galleryEdges[index]!) - decodedBytes(asset, 256) > remaining
      )
        index--;
      const edge = galleryEdges[index]!;
      remaining -= decodedBytes(asset, edge) - decodedBytes(asset, 256);
      return [asset.id, edge] as const;
    }),
  );
}

// While moving, request only visible tiny previews. The virtualizer's existing
// settled signal restores clear tiers/overscan without a second scroll timer.
export function galleryPreviewKeys(
  assets: readonly AssetSummary[],
  tiers: ReadonlyMap<string, number>,
  scrolling: boolean,
): ThumbnailKey[] {
  return assets.slice(0, 240).flatMap((asset) =>
    (scrolling ? [32] : [32, tiers.get(asset.id) ?? 256]).map((edge) => ({
      assetId: asset.id,
      revision: asset.revision,
      edge,
    })),
  );
}
