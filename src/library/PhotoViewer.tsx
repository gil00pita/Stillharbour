import {
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
import { ViewerImage } from './ViewerImage';
import { previewEdges, previewKey, previewUrl, usePreviews } from './previews';

export function PhotoViewer({
  initial,
  pageAssets,
  query,
  status,
  onClose,
  onAnnotate,
  onError,
}: {
  initial: AssetSummary;
  pageAssets: AssetSummary[];
  query: AssetQuery;
  status: LibraryStatus;
  onClose: () => void;
  onAnnotate: (patch: AnnotationPatch, id?: string) => Promise<void>;
  onError: (error: unknown) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const infoButton = useRef<HTMLButtonElement>(null);
  const [asset, setAsset] = useState(initial);
  const [info, setInfo] = useState(false);
  const [filmstrip, setFilmstrip] = useState(false);
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  useEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
    return () => element?.close();
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
    if (
      event.target instanceof HTMLInputElement ||
      event.target instanceof HTMLTextAreaElement ||
      event.target instanceof HTMLSelectElement
    )
      return;
    if (event.key === 'ArrowRight' && context?.data?.next) {
      event.preventDefault();
      setAsset(context.data.next);
    }
    if (event.key === 'ArrowLeft' && context?.data?.previous) {
      event.preventDefault();
      setAsset(context.data.previous);
    }
  }
  async function favorite() {
    setBusy(true);
    setActionError(null);
    try {
      await onAnnotate({ favorite: !asset.favorite, rating: null, tags: null }, asset.id);
      const detail = await libraryData({ kind: 'detail', asset_id: asset.id }, 'detail');
      setAsset((current) => (current.id === detail.asset.id ? detail.asset : current));
    } catch (error) {
      setActionError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <dialog
      className="photo-viewer"
      ref={dialog}
      aria-label="Photo viewer"
      onKeyDown={keyDown}
      onCancel={(event) => {
        event.preventDefault();
        if (info) closeInfo();
        else onClose();
      }}
    >
      <header className="viewer-toolbar">
        <button className="viewer-back" onClick={onClose}>
          Back to gallery
        </button>
        <h2>
          <bdi>{asset.name}</bdi>
        </h2>
        <button aria-pressed={asset.favorite} disabled={busy} onClick={() => void favorite()}>
          {asset.favorite ? 'Favorited' : 'Favorite'}
        </button>
        <button ref={infoButton} aria-pressed={info} onClick={() => setInfo((value) => !value)}>
          Info
        </button>
        <button aria-pressed={filmstrip} onClick={() => setFilmstrip((value) => !value)}>
          Filmstrip
        </button>
        <button
          disabled
          title="Editing original files is unavailable here. Catalog edit recipes require the separate editing workflow."
        >
          Edit
        </button>
      </header>
      {(context?.error || actionError) && (
        <div className="viewer-notice" role="alert">
          {context?.error ?? actionError}
          {context?.error && <button onClick={retryNavigation}>Retry navigation</button>}
        </div>
      )}
      <ViewerStatus asset={asset} status={status} />
      <div className={`viewer-body ${info ? 'with-info' : ''}`}>
        <div className="viewer-main">
          <div className="viewer-photo">
            <button
              className="viewer-previous"
              aria-label="Previous photo"
              disabled={!context?.data?.previous}
              onClick={() => {
                if (context?.data?.previous) setAsset(context.data.previous);
              }}
            >
              Previous
            </button>
            <PhotoStage
              key={`${asset.id}:${asset.revision}`}
              asset={asset}
              ready={ready}
              results={current}
              transportError={previews.error}
              error={
                !ready
                  ? (previews.error ??
                    (current.every((item) => item?.state === 'unavailable')
                      ? (current.find((item) => item?.error)?.error ?? 'Preview unavailable')
                      : null))
                  : null
              }
              onRetry={previews.retry}
            />
            <button
              className="viewer-next"
              aria-label="Next photo"
              disabled={!context?.data?.next}
              onClick={() => {
                if (context?.data?.next) setAsset(context.data.next);
              }}
            >
              Next
            </button>
          </div>
          {filmstrip && (
            <ViewerFilmstrip
              assets={nearby}
              current={asset.id}
              previews={previews}
              onSelect={setAsset}
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
  ready,
  error,
  onRetry,
  results,
  transportError,
}: {
  asset: AssetSummary;
  results: (ThumbnailResult | undefined)[];
  transportError: string | null;
  ready: boolean;
  error: string | null;
  onRetry: () => void;
}) {
  const canvas = useRef<HTMLDivElement>(null);
  const drag = useRef<{ x: number; y: number; left: number; top: number } | null>(null);
  const [zoom, setZoom] = useState(1);
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
          <ViewerImage
            asset={asset}
            results={results}
            transportError={transportError}
            onRetry={onRetry}
          />
        </div>
      </div>
      <div className="viewer-zoom" aria-label="Preview zoom controls">
        <button
          disabled={zoom <= 1 || !ready || Boolean(error)}
          onClick={() => setZoom((value) => Math.max(1, value - 0.25))}
        >
          Zoom out
        </button>
        <label>
          Zoom
          <input
            type="range"
            aria-label="Preview zoom"
            min="100"
            max="400"
            step="25"
            value={zoom * 100}
            disabled={!ready || Boolean(error)}
            onChange={(event) => setZoom(Number(event.target.value) / 100)}
          />
        </label>
        <output>{Math.round(zoom * 100)}% of fit</output>
        <button
          disabled={zoom >= 4 || !ready || Boolean(error)}
          onClick={() => setZoom((value) => Math.min(4, value + 0.25))}
        >
          Zoom in
        </button>
        <button onClick={() => setZoom(1)}>Fit</button>
      </div>
    </div>
  );
}

function ViewerStatus({ asset, status }: { asset: AssetSummary; status: LibraryStatus }) {
  const root = status.roots.find((item) => item.id === asset.rootId);
  const offline =
    !root || !['online', 'scanning'].includes(root.availability) || asset.presence !== 'present';
  return (
    <p className="viewer-status">
      {offline
        ? `Cached preview — original ${asset.presence !== 'present' ? asset.presence : (root?.availability ?? 'availability unknown')}`
        : 'Read-only cached preview'}{' '}
      · up to {VIEWER_EDGE.toLocaleString()} px, not a full-resolution original
    </p>
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
  return <img src={previewUrl(asset, edge)} alt="" onError={() => setFailed(true)} />;
}
