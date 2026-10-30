import { usePhotoMotion, type PhotoOrigin } from './viewerMotion';
import { CachedPreviewImage } from './CachedPreviewImage';
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
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { nearbyPhotos, useViewerNeighbors } from './useViewerNeighbors';
import { Inspector } from './Inspector';
import { OriginalPhoto } from './OriginalPhoto';
import { FavoriteHeart } from './FavoriteHeart';
import { ViewerControls } from './ViewerControls';
import { previewEdges, previewKey, previewUrl, usePreviews } from './previews';

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
    <dialog
      open
      className="photo-viewer"
      aria-modal="false"
      ref={dialog}
      aria-label="Photo viewer"
      onKeyDown={keyDown}
    >
      <header className="viewer-toolbar">
        <h2>
          <bdi>{asset.name}</bdi>
        </h2>
        <button
          disabled
          title="Editing original files is unavailable here. Catalog edit recipes require the separate editing workflow."
        >
          Edit
        </button>
        <button
          className="viewer-back"
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
        </button>
      </header>
      {notice && (
        <div className="viewer-notice" role="alert">
          {notice}
          {context?.error && <button onClick={retryNavigation}>Retry navigation</button>}
        </div>
      )}
      <div className={`viewer-body ${info ? 'with-info' : ''}`}>
        <div className="viewer-main">
          <div className="viewer-photo">
            <button
              className="viewer-previous"
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
            </button>
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
            <button
              className="viewer-next"
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
            </button>
            <div className="viewer-actions">
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
            </div>
          </div>
          {filmstrip && (
            <ViewerFilmstrip
              assets={nearby}
              current={asset.id}
              previews={previews}
              onSelect={navigate}
            />
          )}
        </div>
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
      </div>
    </dialog>
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
    <div className="photo-stage">
      <div
        className={`viewer-canvas ${zoom > 1 ? 'is-zoomed' : ''}`}
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
        <div
          className="viewer-zoom-plane"
          style={{ width: `${zoom * 100}%`, height: `${zoom * 100}%` }}
        >
          <div className="viewer-image-frame" ref={frame}>
            <OriginalPhoto
              onReady={onOriginalReady}
              initialSrc={origin?.src}
              asset={asset}
              results={results}
              transportError={transportError}
              onRetry={onRetry}
            />
          </div>
        </div>
      </div>
    </div>
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
    <nav className="viewer-filmstrip" aria-label="Nearby photos">
      {assets.map((item) => (
        <button
          key={item.id}
          aria-label={`View ${item.name}`}
          aria-current={item.id === current ? 'true' : undefined}
          onClick={() => onSelect(item)}
        >
          <FilmstripImage asset={item} previews={previews} />
        </button>
      ))}
    </nav>
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
    return <span>Preview unavailable</span>;
  if (!edge) return <span>Preparing preview…</span>;
  return (
    <CachedPreviewImage src={previewUrl(asset, edge)} alt="" onError={() => setFailed(true)} />
  );
}
