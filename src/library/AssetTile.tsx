import { useState, type KeyboardEvent } from 'react';
import type { AssetSummary, ThumbnailResult } from '../bindings';
import type { ViewMode } from './model';
import { previewEdges, previewUrl } from './previews';
import { TextListRow } from './TextList';
import type { TextColumn } from './textColumns';

type AssetTileProps = {
  asset: AssetSummary;
  columns: readonly TextColumn[];
  mode: ViewMode;
  width: number | undefined;
  imageHeight: number;
  selected: boolean;
  selectionActive: boolean;
  frozen: boolean;
  offline: boolean;
  edge: number;
  fit: 'contain' | 'cover';
  preview: ThumbnailResult | undefined;
  tinyPreview?: ThumbnailResult | undefined;
  onSelect: (id: string, range: boolean) => void;
  onOpen: (id: string, opener: HTMLButtonElement) => void;
  onNavigate: (event: KeyboardEvent<HTMLElement>) => void;
};

export function AssetTile(props: AssetTileProps) {
  return props.mode === 'list' ? (
    <TextListRow {...props} />
  ) : (
    <ImageTile {...props} mode={props.mode} />
  );
}

function ImageTile({
  asset,
  mode,
  width,
  imageHeight,
  selected,
  selectionActive,
  frozen,
  offline,
  edge,
  fit,
  preview,
  tinyPreview,
  onSelect,
  onOpen,
  onNavigate,
}: Omit<AssetTileProps, 'mode'> & { mode: Exclude<ViewMode, 'list'> }) {
  return (
    <article
      data-asset-id={asset.id}
      data-date={asset.date}
      className={`asset ${selected ? 'is-selected' : ''}`}
      style={{ width }}
    >
      <input
        className="asset-check"
        type="checkbox"
        aria-label={`Select ${asset.name}`}
        checked={selected}
        disabled={frozen}
        title={
          frozen
            ? 'Individual selection is unavailable during a batch action or an all-results selection.'
            : undefined
        }
        onChange={(event) =>
          onSelect(asset.id, Boolean((event.nativeEvent as MouseEvent).shiftKey))
        }
        onKeyDown={onNavigate}
      />
      <button
        className="asset-open"
        aria-label={`Open ${asset.name}`}
        onKeyDown={onNavigate}
        onClick={(event) => {
          if (event.shiftKey || selectionActive) {
            if (!frozen) onSelect(asset.id, event.shiftKey);
          } else onOpen(asset.id, event.currentTarget);
        }}
      >
        <TileImage
          key={`${asset.revision}:${edge}`}
          asset={asset}
          tinyPreview={tinyPreview}
          imageHeight={imageHeight}
          preview={preview}
          edge={edge}
          fit={fit}
          offline={offline}
        />
        {mode !== 'gallery' && (
          <span className="asset-name">
            <bdi>{asset.name}</bdi>
          </span>
        )}
      </button>
    </article>
  );
}

export function TileImage({
  asset,
  imageHeight,
  preview,
  tinyPreview,
  edge,
  fit,
  offline,
}: {
  asset: AssetSummary;
  imageHeight: number;
  preview: ThumbnailResult | undefined;
  tinyPreview?: ThumbnailResult | undefined;
  edge: number;
  fit: 'contain' | 'cover';
  offline: boolean;
}) {
  const [imageFailed, setImageFailed] = useState(false);
  const [failedFallbacks, setFailedFallbacks] = useState<ReadonlySet<number>>(new Set());
  const [clearLoaded, setClearLoaded] = useState(false);
  const fallbackEdge = previewEdges([preview, tinyPreview], failedFallbacks).find(
    (size) => size < edge,
  );
  const tinyReady = fallbackEdge !== undefined;
  const unavailable = imageFailed || preview?.state === 'unavailable';
  const status = imageFailed
    ? 'Cached preview failed to load'
    : (preview?.error ?? asset.previewError ?? 'Preview unavailable');
  return (
    <span className="photo-area" style={{ height: imageHeight }}>
      <span
        className={`photo-layers t-skel ${clearLoaded && !imageFailed ? 'is-revealed' : ''} ${document.visibilityState === 'hidden' ? 'without-motion' : ''}`}
      >
        <span className="t-skel-skeleton">
          {tinyReady ? (
            <img
              src={previewUrl(asset, fallbackEdge!)}
              alt=""
              decoding="async"
              style={{ objectFit: fit }}
              onError={() =>
                setFailedFallbacks((previous) => new Set([...previous, fallbackEdge!]))
              }
              title={unavailable ? `Smaller preview · ${status}` : undefined}
            />
          ) : (
            <span className="preview-fallback" title={status}>
              {unavailable && tinyPreview?.state !== 'pending' ? (
                <>
                  <bdi>{asset.name}</bdi>
                  <span>{status}</span>
                </>
              ) : (
                'Preparing preview…'
              )}
            </span>
          )}
        </span>
        {(preview?.state === 'ready' || clearLoaded) && !imageFailed && (
          <img
            className="t-skel-content"
            src={previewUrl(asset, edge)}
            alt=""
            decoding="async"
            style={{ objectFit: fit }}
            onLoad={() => setClearLoaded(true)}
            onError={() => {
              setImageFailed(true);
              setClearLoaded(false);
            }}
          />
        )}
      </span>
      <PhotoBadges asset={asset} offline={offline} unavailable={unavailable} />
    </span>
  );
}

function PhotoBadges({
  asset,
  offline,
  unavailable,
}: {
  asset: AssetSummary;
  offline: boolean;
  unavailable: boolean;
}) {
  return (
    <>
      {(offline || asset.presence !== 'present') && (
        <span className="asset-badge">{offline ? 'Offline original' : asset.presence}</span>
      )}
      {asset.favorite && !unavailable && (
        <span className="favorite-badge" title="Favorite" aria-label="Favorite">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z" />
          </svg>
        </span>
      )}
    </>
  );
}
