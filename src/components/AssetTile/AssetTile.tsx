import { Text, chakra, Button } from '@chakra-ui/react';
import {
  usePhotoAlbumMenu,
  usePhotoAlbumKeys,
  usePhotoSelection,
} from '@/components/PhotoAlbumActions';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { useState, type KeyboardEvent } from 'react';
import type { AssetSummary, ThumbnailResult } from '../../bindings';
import type { ViewMode } from '../../library/model';
import { previewEdges, previewUrl } from '../../library/previews';
import { TextListRow } from '@/components/TextList';
import type { TextColumn } from '../../library/textColumns';

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
  onRetry?: () => void;
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
  onRetry,
}: Omit<AssetTileProps, 'mode'> & { mode: Exclude<ViewMode, 'list'> }) {
  const contextMenu = usePhotoAlbumMenu(asset);
  const contextKeys = usePhotoAlbumKeys(asset);
  const selection = usePhotoSelection();
  const selecting = selection?.active ?? selectionActive;
  const [imageFailure, setImageFailure] = useState(false);
  return (
    <chakra.article
      onContextMenu={contextMenu}
      onKeyDown={contextKeys}
      data-asset-id={asset.id}
      data-date={asset.date}
      className={`asset ${selected ? 'is-selected' : ''}`}
      style={{ width }}
    >
      {(!selection || selecting) && (
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
      )}
      <Button
        unstyled
        type="submit"
        className="asset-open"
        aria-label={`Open ${asset.name}`}
        onKeyDown={onNavigate}
        onClick={(event) => {
          if (event.shiftKey || selecting) {
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
          onFailure={() => setImageFailure(true)}
        />
        {mode !== 'gallery' && (
          <Text as="span" className="asset-name">
            <bdi>{asset.name}</bdi>
          </Text>
        )}
      </Button>
      {imageFailure && onRetry && (
        <Button
          unstyled
          type="submit"
          className="asset-preview-retry"
          onClick={onRetry}
          aria-label={`Retry preview for ${asset.name}`}
        >
          Retry preview
        </Button>
      )}
    </chakra.article>
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
  onFailure,
}: {
  asset: AssetSummary;
  imageHeight: number;
  preview: ThumbnailResult | undefined;
  tinyPreview?: ThumbnailResult | undefined;
  edge: number;
  fit: 'contain' | 'cover';
  offline: boolean;
  onFailure?: () => void;
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
    <Text as="span" className="photo-area" style={{ height: imageHeight }}>
      <Text
        as="span"
        className={`photo-layers t-skel ${clearLoaded && !imageFailed ? 'is-revealed' : ''} ${document.visibilityState === 'hidden' ? 'without-motion' : ''}`}
      >
        <Text as="span" className="t-skel-skeleton">
          {tinyReady ? (
            <CachedPreviewImage
              className={fallbackEdge === 32 ? 'photo-tiny-preview' : undefined}
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
            <Text as="span" className="preview-fallback" title={status}>
              {unavailable && tinyPreview?.state !== 'pending' ? (
                <>
                  <bdi>{asset.name}</bdi>
                  <Text as="span">{status}</Text>
                </>
              ) : (
                'Preparing preview…'
              )}
            </Text>
          )}
        </Text>
        {(preview?.state === 'ready' || clearLoaded) && !imageFailed && (
          <CachedPreviewImage
            className="t-skel-content"
            src={previewUrl(asset, edge)}
            alt=""
            decoding="async"
            style={{ objectFit: fit }}
            onLoad={() => setClearLoaded(true)}
            onError={() => {
              setImageFailed(true);
              setClearLoaded(false);
              onFailure?.();
            }}
          />
        )}
      </Text>
      <PhotoBadges asset={asset} offline={offline} unavailable={unavailable} />
    </Text>
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
        <Text as="span" className="asset-badge">
          {offline ? 'Offline original' : asset.presence}
        </Text>
      )}
      {asset.favorite && !unavailable && (
        <Text as="span" className="favorite-badge" title="Favorite" aria-label="Favorite">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z" />
          </svg>
        </Text>
      )}
    </>
  );
}
