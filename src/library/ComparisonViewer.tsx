import { useEffect, useRef, useState } from 'react';
import type { AssetSummary, LibraryStatus } from '../bindings';
import { previewEdges, previewKey, previewUrl, usePreviews } from './previews';

export function ComparisonViewer({
  assets,
  status,
  onClose,
  onError,
}: {
  assets: AssetSummary[];
  status: LibraryStatus;
  onClose: () => void;
  onError: (error: unknown) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const keys = assets.map((asset) => ({ assetId: asset.id, revision: asset.revision, edge: 512 }));
  useEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, []);
  const previews = usePreviews(keys, onError);
  return (
    <dialog
      className="comparison-viewer"
      ref={dialog}
      aria-label="Compare selected photos"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header className="viewer-toolbar">
        <h2>Compare {assets.length} selected photos</h2>
        <button onClick={onClose}>Back to gallery</button>
      </header>
      <p className="viewer-status">
        Visual comparison does not establish exact duplicates. Only these explicitly selected photos
        are shown. Cached previews are limited to 512 px.
      </p>
      {previews.error && (
        <div role="alert">
          {previews.error}
          <button onClick={previews.retry}>Retry previews</button>
        </div>
      )}
      <div className="comparison-images">
        {assets.map((asset) => {
          const preview =
            previews.results[
              previewKey({ assetId: asset.id, revision: asset.revision, edge: 512 })
            ];
          const edge = previewEdges([preview])[0];
          return (
            <figure key={asset.id}>
              <div>
                {edge ? (
                  <ComparisonImage
                    key={`${asset.id}:${asset.revision}:${edge}`}
                    asset={asset}
                    edge={edge}
                  />
                ) : (
                  <p>{preview?.error ?? 'Preparing cached preview…'}</p>
                )}
              </div>
              <figcaption>
                <bdi>{asset.name}</bdi> · Catalog: {asset.presence} · Original library:{' '}
                {status.roots.find((root) => root.id === asset.rootId)?.availability ??
                  'availability unknown'}
              </figcaption>
            </figure>
          );
        })}
      </div>
    </dialog>
  );
}

function ComparisonImage({ asset, edge }: { asset: AssetSummary; edge: number }) {
  const [failed, setFailed] = useState(false);
  if (failed)
    return <p role="status">Cached preview failed to load. The original has not been changed.</p>;
  return <img src={previewUrl(asset, edge)} alt={asset.name} onError={() => setFailed(true)} />;
}
