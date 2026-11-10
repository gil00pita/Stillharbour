import { Box, Heading, Text, chakra, Button, IconButton } from '@chakra-ui/react';
import { usePhotoMotion, type PhotoOrigin } from '../../library/viewerMotion';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from 'react';
import {
  VIEWER_EDGE,
  type AnnotationPatch,
  type AssetQuery,
  type AssetSummary,
  type LibraryStatus,
  type ThumbnailResult,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { nearbyPhotos, useViewerNeighbors } from '@/hooks/use-viewer-neighbors';
import { Inspector } from '@/components/Inspector';
import { OriginalPhoto } from '@/components/OriginalPhoto';
import { FavoriteHeart } from '@/components/FavoriteHeart';
import { ViewerControls } from '@/components/ViewerControls';
import { previewEdges, previewKey, previewUrl, usePreviews } from '../../library/previews';

export function PhotoViewer({
  initial,
  pageAssets,
  query,
  status,
  onClose,
  onAnnotate,
  onError,
  origin,
}: {
  initial: AssetSummary;
  origin?: PhotoOrigin | undefined;
  pageAssets: AssetSummary[];
  query: AssetQuery;
  status: LibraryStatus;
  onClose: () => void;
  onAnnotate: (patch: AnnotationPatch, id?: string) => Promise<void>;
  onError: (error: unknown) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = useRef<(() => void) | null>(null);
  const closeButton = useRef<HTMLButtonElement>(null);
  function requestClose() {
    close.current?.();
  }
  const infoButton = useRef<HTMLButtonElement>(null);
  const [asset, setAsset] = useState(initial);
  const assetKey = `${asset.id}:${asset.revision}`;
  const [zoomState, setZoom] = useState({ key: assetKey, value: 1 });
  const zoom = zoomState.key === assetKey ? zoomState.value : 1;
  const [originalReadyKey, setOriginalReadyKey] = useState<string | null>(null);
  const originalReady = originalReadyKey === assetKey;
  const originalReadiness = useCallback(
    (ready: boolean) => setOriginalReadyKey(ready ? assetKey : null),
    [assetKey],
  );
  const currentAssetIdRef = useRef(initial.id);
  useLayoutEffect(() => {
    currentAssetIdRef.current = asset.id;
  }, [asset.id]);
  const [info, setInfo] = useState(false);
  const [filmstrip, setFilmstrip] = useState(false);
  const [busy, setBusy] = useState(false);
  const [celebrate, setCelebrate] = useState<string | null>(null);
  const stopCelebration = useCallback(() => setCelebrate(null), []);
  function navigate(next: AssetSummary) {
    setCelebrate(null);
    setActionError(null);
    setZoom({ key: `${next.id}:${next.revision}`, value: 1 });
    setOriginalReadyKey(null);
    setAsset(next);
  }
  const [actionError, setActionError] = useState<{ id: string; message: string } | null>(null);
  useEffect(() => {
    closeButton.current?.focus({ preventScroll: true });
  }, []);
  const { context, retry: retryNavigation } = useViewerNeighbors(asset.id, query);
  const nearby = nearbyPhotos(pageAssets, asset, context?.data);
  const keys = [
    ...[VIEWER_EDGE, 512, 32].map((edge) => ({
      assetId: asset.id,
      revision: asset.revision,
      edge,
    })),
    ...(filmstrip
      ? nearby.map((item) => ({ assetId: item.id, revision: item.revision, edge: 256 }))
      : []),
  ];
  const previews = usePreviews(keys, onError);
  const current = keys.slice(0, 3).map((key) => previews.results[previewKey(key)]);
  const ready = previewEdges(current).length > 0;
  function closeInfo() {
    setInfo(false);
    infoButton.current?.focus();
  }
  function keyDown(event: KeyboardEvent<HTMLDialogElement>) {
    if (event.key === 'Escape') {
      event.preventDefault();
      if (info) closeInfo();
      else requestClose();
      return;
    }
    if (
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      event.target instanceof HTMLSelectElement
    )
      return;
    if (event.key === 'ArrowRight' && context?.data?.next) {
      event.preventDefault();
      navigate(context.data.next);
    }
    if (event.key === 'ArrowLeft' && context?.data?.previous) {
      event.preventDefault();
      navigate(context.data.previous);
    }
  }
  async function favorite() {
    setBusy(true);
    setActionError(null);
    try {
      await onAnnotate({ favorite: !asset.favorite, rating: null, tags: null }, asset.id);
      const detail = await libraryData({ kind: 'detail', asset_id: asset.id }, 'detail');
      setAsset((current) => (current.id === detail.asset.id ? detail.asset : current));
      if (detail.asset.favorite && currentAssetIdRef.current === detail.asset.id)
        setCelebrate(detail.asset.id);
    } catch (error) {
      setActionError({ id: asset.id, message: errorMessage(error) });
    } finally {
      setBusy(false);
    }
  }
  const notice = context?.error ?? (actionError?.id === asset.id ? actionError.message : null);
  return (
    <chakra.dialog
      open
      className="photo-viewer"
      css={{
        position: 'absolute',
        inset: '0',
        width: '100%',
        height: '100%',
        maxWidth: 'none',
        maxHeight: 'none',
        padding: '0',
        margin: '0',
        border: '0',
        color: 'var(--stillharbour-colors-fg, {colors.fg})',
        background: '{colors.bg}',
        '&[open]': { display: 'flex', flexDirection: 'column' },
        '&::backdrop': { background: '{colors.bg.sidebar}' },
        '& .inspector': {
          width: '320px',
          flexShrink: '0',
          padding: '18px',
          background: '{colors.bg.panel}',
        },
        '@media (max-width: 760px)': {
          '& .inspector': {
            width: '100%',
            maxHeight: '45%',
            minHeight: '0',
            flex: '0 1 45%',
            overflowY: 'auto',
          },
        },
        '--viewer-open-ms': '250',
        '--viewer-close-ms': '150',
        '--viewer-ease': 'cubic-bezier(0.22, 1, 0.36, 1)',
        display: 'flex',
        flexDirection: 'column',
        zIndex: '5',
        overflow: 'hidden',
        "&[data-motion='closing'] .viewer-toolbar": { opacity: '0' },
        "&[data-motion='closing'] .viewer-status": { opacity: '0' },
        "&[data-motion='closing'] .viewer-zoom": { opacity: '0' },
        "&[data-motion='opening'] .viewer-image-frame": { willChange: 'transform, opacity' },
        "&[data-motion='closing'] .viewer-image-frame": { willChange: 'transform, opacity' },
        "&[data-motion='opening'] .viewer-canvas": { overflow: 'visible' },
        "&[data-motion='closing'] .viewer-canvas": { overflow: 'visible' },
        "&[data-motion='closing']": { background: 'transparent' },
        "&[data-motion='closing'] .viewer-previous": { visibility: 'hidden' },
        "&[data-motion='closing'] .viewer-next": { visibility: 'hidden' },
        "&[data-motion='closing'] .viewer-filmstrip": { visibility: 'hidden' },
        "&[data-motion='closing'] .inspector": { visibility: 'hidden' },
        "&[data-motion='closing'] .viewer-actions": { opacity: '0' },
      }}
      aria-modal="false"
      ref={dialog}
      aria-label="Photo viewer"
      onKeyDown={keyDown}
    >
      <chakra.header
        className="viewer-toolbar"
        css={{
          display: 'flex',
          gap: '10px',
          alignItems: 'center',
          minHeight: '64px',
          padding: '12px 20px',
          borderBottom: '1px solid {colors.border}',
          flexShrink: '0',
          '& h2': {
            flex: '1',
            minWidth: '0',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
            fontSize: '13px',
            fontWeight: '500',
            margin: '0',
          },
          '@media (max-width: 760px)': {
            flexWrap: 'wrap',
            padding: '10px',
            gap: '6px',
            '& h2': { flexBasis: '45%' },
            '& button': { padding: '7px 9px !important' },
          },
        }}
      >
        <Heading as="h2" unstyled>
          <bdi>{asset.name}</bdi>
        </Heading>
        <Button
          unstyled
          type="submit"
          disabled
          title="Editing original files is unavailable here. Catalog edit recipes require the separate editing workflow."
        >
          Edit
        </Button>
        <IconButton
          unstyled
          type="submit"
          className="viewer-back"
          css={{
            display: 'grid',
            placeItems: 'center',
            minWidth: '36px',
            minHeight: '36px',
            flexShrink: '0',
          }}
          ref={closeButton}
          onClick={requestClose}
          aria-label="Close photo viewer"
          title="Back to gallery"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            aria-hidden="true"
          >
            <path d="m6 6 12 12M18 6 6 18" />
          </svg>
        </IconButton>
      </chakra.header>
      {notice && (
        <Box
          className="viewer-notice"
          css={{ padding: '10px 20px', fontSize: '12px' }}
          role="alert"
        >
          {notice}
          {context?.error && (
            <Button unstyled type="submit" onClick={retryNavigation}>
              Retry navigation
            </Button>
          )}
        </Box>
      )}
      <Box
        className={`viewer-body ${info ? 'with-info' : ''}`}
        css={{
          display: 'flex',
          flex: '1',
          minHeight: '0',
          position: 'relative',
          '@media (max-width: 760px)': {
            '&.with-info': { flexDirection: 'column' },
            '&.with-info .viewer-main': { minHeight: '214px' },
          },
        }}
      >
        <Box
          className="viewer-main"
          css={{
            display: 'flex',
            flexDirection: 'column',
            minWidth: '0',
            minHeight: '0',
            flex: '1',
          }}
        >
          <Box
            className="viewer-photo"
            css={{ display: 'flex', position: 'relative', flex: '1', minHeight: '0' }}
          >
            <IconButton
              unstyled
              type="submit"
              className="viewer-previous"
              css={{
                position: 'absolute',
                zIndex: '2',
                top: 'max(12px, calc(50% - 64px))',
                display: 'grid',
                placeItems: 'center',
                width: '40px',
                minHeight: '40px',
                padding: '8px',
                left: '12px',
              }}
              aria-label="Previous photo"
              title="Previous photo"
              disabled={!context?.data?.previous}
              onClick={() => {
                if (context?.data?.previous) navigate(context.data.previous);
              }}
            >
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                aria-hidden="true"
              >
                <path d="M19 12H5m6-6-6 6 6 6" />
              </svg>
            </IconButton>
            <PhotoStage
              key={`${asset.id}:${asset.revision}`}
              asset={asset}
              origin={asset.id === initial.id ? origin : undefined}
              closeHandleRef={close}
              onClose={onClose}
              zoom={zoom}
              onOriginalReady={originalReadiness}
              results={current}
              transportError={previews.error}
              onRetry={previews.retry}
            />
            <IconButton
              unstyled
              type="submit"
              className="viewer-next"
              css={{
                position: 'absolute',
                zIndex: '2',
                top: 'max(12px, calc(50% - 64px))',
                display: 'grid',
                placeItems: 'center',
                width: '40px',
                minHeight: '40px',
                padding: '8px',
                right: '12px',
              }}
              aria-label="Next photo"
              title="Next photo"
              disabled={!context?.data?.next}
              onClick={() => {
                if (context?.data?.next) navigate(context.data.next);
              }}
            >
              <svg
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                aria-hidden="true"
              >
                <path d="M5 12h14m-6-6 6 6-6 6" />
              </svg>
            </IconButton>
            <Box
              className="viewer-actions"
              css={{
                '& .viewer-controls button': {
                  display: 'grid',
                  placeItems: 'center',
                  flexShrink: '0',
                  width: '44px',
                  minHeight: '44px',
                  padding: '8px',
                  border: '0',
                  background: 'transparent',
                },
                position: 'absolute',
                inset: 'auto 16px 16px',
                zIndex: '3',
                display: 'flex',
                justifyContent: 'center',
                pointerEvents: 'none',
                '& .viewer-controls': { pointerEvents: 'auto' },
                '& .viewer-controls button.viewer-favorite': {
                  '--like-color': '{colors.fg.secondary}',
                  '--like-fill': '150ms',
                  '--like-pop': '800ms',
                  '--like-pop-ease': 'cubic-bezier(0.22, 1, 0.36, 1)',
                  '--like-ease': 'var(--ui-ease)',
                  display: 'grid',
                  placeItems: 'center',
                  width: '44px',
                  minHeight: '44px',
                  padding: '8px',
                  overflow: 'visible',
                },
                '@media (prefers-reduced-motion: reduce)': {
                  '& .viewer-controls button.viewer-favorite': { '--like-pop': '0ms' },
                },
              }}
            >
              <ViewerControls
                zoom={zoom}
                canZoom={originalReady || ready}
                onZoom={(value) => setZoom({ key: assetKey, value })}
                info={info}
                filmstrip={filmstrip}
                onInfo={() => setInfo((value) => !value)}
                onFilmstrip={() => setFilmstrip((value) => !value)}
                infoButtonRef={infoButton}
                favorite={
                  <FavoriteHeart
                    key={asset.id}
                    favorite={asset.favorite}
                    busy={busy}
                    animate={celebrate === asset.id}
                    onClick={() => void favorite()}
                    onAnimationEnd={stopCelebration}
                  />
                }
              />
            </Box>
          </Box>
          {filmstrip && (
            <ViewerFilmstrip
              assets={nearby}
              current={asset.id}
              previews={previews}
              onSelect={navigate}
            />
          )}
        </Box>
        {info && (
          <Inspector
            key={asset.id}
            id={asset.id}
            revision={status.revision}
            onClose={closeInfo}
            onAnnotate={onAnnotate}
            onError={onError}
          />
        )}
      </Box>
    </chakra.dialog>
  );
}

function PhotoStage({
  asset,
  zoom,
  onOriginalReady,
  onRetry,
  results,
  transportError,
  origin,
  onClose,
  closeHandleRef,
}: {
  asset: AssetSummary;
  origin: PhotoOrigin | undefined;
  onClose: () => void;
  closeHandleRef: import('react').RefObject<(() => void) | null>;
  results: (ThumbnailResult | undefined)[];
  transportError: string | null;
  zoom: number;
  onOriginalReady: (ready: boolean) => void;
  onRetry: () => void;
}) {
  const canvas = useRef<HTMLDivElement>(null);
  const frame = usePhotoMotion(asset, origin, onClose, closeHandleRef);
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    const element = canvas.current;
    if (element) {
      element.scrollLeft = (element.scrollWidth - element.clientWidth) / 2;
      element.scrollTop = (element.scrollHeight - element.clientHeight) / 2;
    }
  }, [zoom]);
  function pan(event: PointerEvent<HTMLDivElement>) {
    if (!drag.current) return;
    event.currentTarget.scrollLeft = drag.current.left - (event.clientX - drag.current.x);
    event.currentTarget.scrollTop = drag.current.top - (event.clientY - drag.current.y);
  }
  return (
    <Box
      className="photo-stage"
      css={{ flex: '1', minWidth: '0', minHeight: '0', display: 'flex', flexDirection: 'column' }}
    >
      <Box
        className={`viewer-canvas ${zoom > 1 ? 'is-zoomed' : ''}`}
        css={{
          '&:focus': { outline: 'none' },
          flex: '1',
          minWidth: '0',
          minHeight: '0',
          overflow: 'auto',
          position: 'relative',
          overscrollBehavior: 'contain',
          '&.is-zoomed': { cursor: 'grab', touchAction: 'none' },
          '&.is-zoomed:active': { cursor: 'grabbing' },
        }}
        ref={canvas}
        tabIndex={0}
        aria-label="Photo preview, drag to pan when zoomed"
        onPointerDown={(event) => {
          if (zoom <= 1) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          drag.current = {
            x: event.clientX,
            y: event.clientY,
            left: event.currentTarget.scrollLeft,
            top: event.currentTarget.scrollTop,
          };
        }}
        onPointerMove={pan}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      >
        <Box
          className="viewer-zoom-plane"
          css={{
            minWidth: '100%',
            minHeight: '100%',
            '& img': { width: '100%', height: '100%', objectFit: 'contain', display: 'block' },
            display: 'grid',
            placeItems: 'center',
          }}
          style={{ width: `${zoom * 100}%`, height: `${zoom * 100}%` }}
        >
          <Box
            className="viewer-image-frame"
            css={{ flex: 'none', position: 'relative', transformOrigin: 'top left' }}
            ref={frame}
          >
            <OriginalPhoto
              onReady={onOriginalReady}
              initialSrc={origin?.src}
              asset={asset}
              results={results}
              transportError={transportError}
              onRetry={onRetry}
            />
          </Box>
        </Box>
      </Box>
    </Box>
  );
}

function ViewerFilmstrip({
  assets,
  current,
  previews,
  onSelect,
}: {
  assets: AssetSummary[];
  current: string;
  previews: ReturnType<typeof usePreviews>;
  onSelect: (asset: AssetSummary) => void;
}) {
  return (
    <chakra.nav
      className="viewer-filmstrip"
      css={{
        margin: '0',
        padding: '10px',
        display: 'flex',
        justifyContent: 'center',
        gap: '4px',
        overflowX: 'auto',
        flexShrink: '0',
        borderTop: '1px solid {colors.border}',
        '& button': {
          padding: '2px !important',
          flex: '0 0 64px',
          width: '64px',
          height: '64px',
          borderRadius: '0 !important',
        },
        '& button[aria-current]': { borderColor: '{colors.brand.solid} !important' },
        '& img': { width: '100%', height: '100%', objectFit: 'contain' },
        '& span': { fontSize: '9px' },
      }}
      aria-label="Nearby photos"
    >
      {assets.map((item) => (
        <Button
          unstyled
          type="submit"
          key={item.id}
          aria-label={`View ${item.name}`}
          aria-current={item.id === current ? 'true' : undefined}
          onClick={() => onSelect(item)}
        >
          <FilmstripImage asset={item} previews={previews} />
        </Button>
      ))}
    </chakra.nav>
  );
}
function FilmstripImage({
  asset,
  previews,
}: {
  asset: AssetSummary;
  previews: ReturnType<typeof usePreviews>;
}) {
  const [failed, setFailed] = useState(false);
  const result =
    previews.results[previewKey({ assetId: asset.id, revision: asset.revision, edge: 256 })];
  const edge = previewEdges([result])[0];
  if (failed || (!edge && (result?.state === 'unavailable' || previews.error)))
    return <Text as="span">Preview unavailable</Text>;
  if (!edge) return <Text as="span">Preparing preview…</Text>;
  return (
    <CachedPreviewImage src={previewUrl(asset, edge)} alt="" onError={() => setFailed(true)} />
  );
}
