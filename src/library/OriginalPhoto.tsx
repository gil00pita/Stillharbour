import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { ORIGINAL_IMAGE_POLICY, type AssetSummary, type ThumbnailResult } from '../bindings';
import { CachedPreviewImage } from './CachedPreviewImage';
import { LoadingSheen, ViewerImage } from './ViewerImage';

export function OriginalPhoto({
  asset,
  results,
  transportError,
  initialSrc,
  onRetry,
  onReady,
}: {
  asset: AssetSummary;
  results: (ThumbnailResult | undefined)[];
  transportError: string | null;
  initialSrc: string | undefined;
  onRetry: () => void;
  onReady: (ready: boolean) => void;
}) {
  const [state, setState] = useState<'loading' | 'loaded' | 'failed'>('loading');
  const [attempt, setAttempt] = useState(0);
  const source = `${convertFileSrc('', 'original')}${asset.id}/${asset.revision}/${ORIGINAL_IMAGE_POLICY}?request=${attempt}`;
  function retryOriginal() {
    onReady(false);
    setState('loading');
    setAttempt((value) => value + 1);
  }
  return (
    <div
      className={`original-photo ${state === 'loaded' ? 'has-original' : ''} ${state === 'failed' ? 'has-failed' : ''}`}
      aria-busy={state === 'loading'}
    >
      <div className="original-fallback" aria-hidden={state === 'loaded'}>
        <ViewerImage
          asset={asset}
          results={results}
          transportError={transportError}
          initialSrc={initialSrc}
          onRetry={onRetry}
        />
      </div>
      {state !== 'failed' && (
        <CachedPreviewImage
          key={attempt}
          className="original-image"
          src={source}
          aria-hidden={state !== 'loaded'}
          alt={asset.name}
          draggable={false}
          onLoad={() => {
            setState('loaded');
            onReady(true);
          }}
          onError={() => {
            setState('failed');
            onReady(false);
          }}
        />
      )}
      {state === 'loading' && (
        <>
          <LoadingSheen />
          <p className="original-notice" role="status">
            Loading original…
          </p>
        </>
      )}
      {state === 'failed' && (
        <p
          className="original-notice"
          role="status"
          title="Original viewing supports static JPEG, PNG and WebP, plus HEIC/HEIF on compatible macOS webviews, up to 32 MiB and 32 megapixels. Offline originals must be reconnected."
        >
          Original unavailable · showing a preview.{' '}
          <button onClick={retryOriginal}>Retry original</button>
        </p>
      )}
    </div>
  );
}
