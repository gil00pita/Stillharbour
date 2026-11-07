import { Box, Text, chakra, Button } from '@chakra-ui/react';
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
import { ThumbnailFavorite } from './components/ThumbnailFavorite';
import type { TextColumn } from '../../library/textColumns';
import { thumbnailControlStyles, thumbnailMarkStyles } from './AssetTile.recipe';

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
  onFavorite?: ((id: string, favorite: boolean) => Promise<void>) | undefined;
  favoriteBusy?: boolean | undefined;
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
  onFavorite,
  favoriteBusy = false,
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
      data-narrow-controls={(width !== undefined && width < 86) || undefined}
      className={`asset ${selected ? 'is-selected' : ''}`}
      css={{
        position: 'relative',
        borderRadius: '5px',
        minWidth: '0',
        flexShrink: '0',
        '& .photo-favorite': { opacity: asset.favorite ? 1 : 0 },
        '& .asset-check, & .thumbnail-selection': {
          opacity: selecting || selected ? 1 : 0,
          transition: 'none',
        },
        '&:hover .asset-check, &:hover .thumbnail-selection': {
          opacity: 1,
          transition:
            selecting || selected ? 'none' : 'opacity 150ms cubic-bezier(0.16, 1, 0.3, 1)',
        },
        '&:focus-within .asset-check, &:focus-within .thumbnail-selection': {
          opacity: 1,
          transition: 'none',
        },
        _motionReduce: { '& .asset-check, & .thumbnail-selection': { transition: 'none' } },
        '& .favorite-badge': { display: onFavorite ? 'none' : 'grid' },
        '&:hover .photo-favorite, &:focus-within .photo-favorite': { opacity: 1 },
        '&[data-narrow-controls] .photo-favorite': {
          top: 'calc({sizes.control} + {spacing.2})',
          left: '2',
          right: 'auto',
        },
        '&.is-selected': {
          outline: '2px solid {colors.brand.solid}',
          outlineOffset: '-2px',
          background: '{colors.brand.selected}',
        },
        '&.is-active': { boxShadow: 'inset 0 -2px 0 {colors.brand.solid}' },
      }}
      style={{ width }}
    >
      <chakra.input
        className="asset-check"
        css={[
          thumbnailControlStyles,
          {
            left: '2',
            'input&': {
              appearance: 'none',
              width: 'control',
              height: 'control',
              border: 0,
              background: 'transparent',
            },
            '&:hover + .thumbnail-selection > span': { background: 'photo.controlHover' },
            '&:disabled + .thumbnail-selection > span': { opacity: 0.55 },
          },
        ]}
        type="checkbox"
        aria-label={`Select ${asset.name}`}
        checked={selected}
        disabled={frozen || selection?.disabled}
        title={
          frozen
            ? 'Individual selection is unavailable during a batch action or an all-results selection.'
            : undefined
        }
        onChange={(event) => {
          if (selection && !selecting) selection.onSelect(asset);
          else onSelect(asset.id, Boolean((event.nativeEvent as MouseEvent).shiftKey));
        }}
        onKeyDown={onNavigate}
      />
      <Box
        as="span"
        className="thumbnail-selection"
        aria-hidden="true"
        css={[thumbnailControlStyles, { left: '2', pointerEvents: 'none' }]}
      >
        <Box as="span" className="thumbnail-control-mark" css={thumbnailMarkStyles}>
          <svg viewBox="0 0 24 24" fill="none" aria-hidden="true" focusable="false">
            <rect
              x="3"
              y="3"
              width="18"
              height="18"
              rx="3"
              stroke="currentColor"
              strokeWidth="2"
              fill={selected ? 'currentColor' : 'none'}
            />
            {selected && (
              <path
                className="selection-tick"
                d="m7 12 3 3 7-7"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            )}
          </svg>
        </Box>
      </Box>
      <Button
        unstyled
        type="submit"
        className="asset-open"
        css={{
          border: '0',
          background: 'transparent',
          padding: '0',
          color: 'inherit',
          width: '100%',
          textAlign: 'left',
          fontSize: '12px',
          height: '100%',
          display: 'flex',
          flexDirection: 'column',
        }}
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
          <Text
            as="span"
            className="asset-name"
            css={{
              display: 'flex',
              gap: '8px',
              alignItems: 'center',
              padding: '10px 4px',
              width: '100%',
              minWidth: '0',
              '& bdi': { overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' },
            }}
          >
            <bdi>{asset.name}</bdi>
          </Text>
        )}
      </Button>
      {onFavorite && (
        <ThumbnailFavorite asset={asset} disabled={frozen || favoriteBusy} onSave={onFavorite} />
      )}
      {imageFailure && onRetry && (
        <Button
          unstyled
          type="submit"
          className="asset-preview-retry"
          css={{
            position: 'absolute',
            zIndex: '3',
            right: '6px',
            bottom: '6px',
            padding: '6px 9px',
            fontSize: '11px',
          }}
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
    <Text
      as="span"
      className="photo-area"
      css={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: '100%',
        overflow: 'hidden',
        background: '{colors.bg.hover}',
        borderRadius: '4px',
        '& img': { width: '100%', height: '100%', objectFit: 'contain' },
        '&[data-unavailable]::after': {
          content: 'attr(data-unavailable)',
          color: '{colors.fg.muted}',
          fontSize: '11px',
          padding: '12px',
        },
      }}
      style={{ height: imageHeight }}
    >
      <Text
        as="span"
        className={`photo-layers t-skel ${clearLoaded && !imageFailed ? 'is-revealed' : ''} ${document.visibilityState === 'hidden' ? 'without-motion' : ''}`}
        css={{
          isolation: 'isolate',
          overflow: 'hidden',
          display: 'block',
          width: '100%',
          height: '100%',
          '& .preview-fallback': {
            display: 'flex',
            height: '100%',
            alignItems: 'center',
            justifyContent: 'center',
          },
          '&.without-motion .t-skel-skeleton': { transition: 'none' },
          '&.without-motion .t-skel-content': { transition: 'none' },
          position: 'relative',
          '&.is-revealed .t-skel-skeleton': { opacity: '0', filter: 'blur(var(--reveal-blur))' },
          '&.is-revealed .t-skel-content': { opacity: '1', filter: 'blur(0)' },
          '&.is-resetting .t-skel-skeleton': { transition: 'none !important' },
          '&.is-resetting .t-skel-content': { transition: 'none !important' },
        }}
      >
        <Text
          as="span"
          className="t-skel-skeleton"
          css={{
            position: 'absolute',
            inset: '0',
            zIndex: '1',
            opacity: '1',
            filter: 'blur(0)',
            transition:
              'opacity var(--reveal-dur) var(--reveal-ease),\n    filter var(--reveal-dur) var(--reveal-ease)',
            '&.is-pulsing > *': {
              animation: 't-skel-pulse var(--pulse-dur) ease-in-out var(--pulse-count)',
            },
            '@media (prefers-reduced-motion: reduce)': {
              transition: 'none !important',
              '&.is-pulsing > *': { animation: 'none !important' },
            },
          }}
        >
          {tinyReady ? (
            <CachedPreviewImage
              className={fallbackEdge === 32 ? 'photo-tiny-preview' : undefined}
              css={{ '&.photo-tiny-preview': { filter: 'blur(8px)' } }}
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
            <Text
              as="span"
              className="preview-fallback"
              css={{
                fontSize: '11px',
                color: '{colors.fg.muted}',
                textAlign: 'center',
                padding: '12px',
                display: 'flex',
                flexDirection: 'column',
                gap: '6px',
                overflowWrap: 'anywhere',
                maxHeight: '100%',
                overflow: 'hidden',
                '& bdi': { fontWeight: '500' },
              }}
              title={status}
            >
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
            css={{
              position: 'absolute',
              inset: '0',
              zIndex: '2',
              opacity: '0',
              filter: 'blur(var(--reveal-blur))',
              transition:
                'opacity var(--reveal-dur) var(--reveal-ease),\n    filter var(--reveal-dur) var(--reveal-ease)',
              '@media (prefers-reduced-motion: reduce)': { transition: 'none !important' },
            }}
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
        <Text
          as="span"
          className="asset-badge"
          css={{
            position: 'absolute',
            bottom: '8px',
            left: '8px',
            maxWidth: 'calc(100% - 16px)',
            background: '{colors.bg.sidebar}',
            padding: '4px 6px',
            borderRadius: '4px',
            fontSize: '10px',
          }}
        >
          {offline ? 'Offline original' : asset.presence}
        </Text>
      )}
      {asset.favorite && !unavailable && (
        <Text
          as="span"
          className="favorite-badge"
          css={{
            position: 'absolute',
            right: '8px',
            top: '8px',
            display: 'grid',
            placeItems: 'center',
            padding: '4px',
            color: '{colors.brand.solid}',
            background: '{colors.bg.sidebar}',
            borderRadius: '4px',
          }}
          title="Favorite"
          aria-label="Favorite"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-3-5.6 3 1.1-6.2L3 9.6l6.2-.9Z" />
          </svg>
        </Text>
      )}
    </>
  );
}
