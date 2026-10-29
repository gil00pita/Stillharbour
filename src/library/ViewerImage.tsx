import { useState } from 'react';
import { VIEWER_EDGE, type AssetSummary, type ThumbnailResult } from '../bindings';
import { previewEdges, previewUrl } from './previews';

export function ViewerImage({
  asset,
  results,
  transportError,
  onRetry,
}: {
  asset: AssetSummary;
  results: (ThumbnailResult | undefined)[];
  transportError: string | null;
  onRetry: () => void;
}) {
  const [failed, setFailed] = useState<ReadonlySet<number>>(new Set());
  const [loaded, setLoaded] = useState(false);
  const available = previewEdges(results, failed);
  const high = available.includes(VIEWER_EDGE);
  const small = available.find((edge) => edge < VIEWER_EDGE);
  const error =
    transportError ??
    results.find((item) => item?.error)?.error ??
    'Cached preview failed to load.';
  function fail(edge: number) {
    setFailed((previous) => new Set([...previous, edge]));
  }
  function retry() {
    setFailed(new Set());
    setLoaded(false);
    onRetry();
  }
  if (!high && !small) {
    const pending = !transportError && results.some((item) => !item || item.state === 'pending');
    return pending ? (
      <p className="viewer-loading" role="status">
        Preparing cached preview…
      </p>
    ) : (
      <div className="viewer-unavailable" role="status">
        <h3>
          <bdi>{asset.name}</bdi>
        </h3>
        <p>{error}</p>
        <p>The original has not been changed.</p>
        <button onClick={retry}>Retry preview</button>
      </div>
    );
  }
  return (
    <div
      className={`viewer-renditions ${loaded && high ? 'is-revealed' : ''} ${document.visibilityState === 'hidden' ? 'without-motion' : ''}`}
    >
      {small && (
        <img
          className="viewer-rendition-small"
          src={previewUrl(asset, small)}
          alt={asset.name}
          draggable={false}
          onError={() => fail(small)}
        />
      )}
      {high && (
        <img
          className="viewer-rendition-high"
          src={previewUrl(asset, VIEWER_EDGE)}
          alt={small ? '' : asset.name}
          draggable={false}
          onLoad={() => setLoaded(true)}
          onError={() => fail(VIEWER_EDGE)}
        />
      )}
      {!high &&
        (transportError || results[0]?.state === 'unavailable' || failed.has(VIEWER_EDGE)) && (
          <p className="viewer-quality" role="status">
            Smaller cached preview ·{' '}
            {failed.has(VIEWER_EDGE) ? 'Higher detail could not be loaded.' : error}{' '}
            <button onClick={retry}>Retry higher detail</button>
          </p>
        )}
    </div>
  );
}
