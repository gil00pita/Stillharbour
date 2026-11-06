import { useCallback, useId, useState } from 'react';
import { Field } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import type { AssetSummary } from '../../bindings';
import { previewEdges, previewKey, previewUrl, usePreviews } from '../../library/previews';
import { errorMessage } from '../../ipc';

export function DuplicateComparison({ assets }: { assets: AssetSummary[] }) {
  const [mode, setMode] = useState('Side by side');
  const [position, setPosition] = useState(50);
  const [failed, setFailed] = useState(new Set<string>());
  const [error, setError] = useState<string | null>(null);
  const id = useId().replaceAll(':', '');
  const onError = useCallback((failure: unknown) => setError(errorMessage(failure)), []);
  const keys = assets
    .slice(0, 2)
    .map((asset) => ({ assetId: asset.id, revision: asset.revision, edge: 512 }));
  const previews = usePreviews(keys, onError);
  const ready =
    assets.length === 2 &&
    assets.every(
      (asset, index) =>
        previewEdges([previews.results[previewKey(keys[index]!)]])[0] && !failed.has(asset.id),
    );
  function image(asset: AssetSummary, index: number) {
    const result = previews.results[previewKey(keys[index]!)];
    const edge = previewEdges([result])[0];
    if (!edge || failed.has(asset.id))
      return (
        <div className="duplicate-image-state" role="status">
          {failed.has(asset.id)
            ? 'Preview failed to load.'
            : (result?.error ?? 'Preparing cached preview…')}
        </div>
      );
    return (
      <img
        src={previewUrl(asset, edge)}
        alt={asset.name}
        draggable={false}
        onError={() => setFailed((previous) => new Set([...previous, asset.id]))}
      />
    );
  }
  return (
    <section className="duplicate-comparison" aria-label="Compare duplicate previews">
      <div className="duplicate-mode-bar" role="group" aria-label="Comparison mode">
        {['Slider', 'Side by side', 'Difference heatmap'].map((item) => (
          <Button key={item} aria-pressed={mode === item} onClick={() => setMode(item)}>
            {item}
          </Button>
        ))}
      </div>
      <p className="duplicate-preview-note">
        Cached previews up to 512 px · visual inspection cannot prove byte equality.
      </p>
      {error && <p role="alert">{error}</p>}
      <div
        className={`duplicate-stage ${mode === 'Side by side' || !ready ? 'duplicate-stage-pair' : 'duplicate-stage-overlay'} ${mode === 'Difference heatmap' && ready ? `duplicate-heat-${id}` : ''}`}
      >
        {assets.map((asset, index) => (
          <figure
            key={asset.id}
            style={
              ready && mode === 'Slider' && index === 1
                ? { clipPath: `inset(0 0 0 ${position}%)` }
                : undefined
            }
            className={ready && mode === 'Difference heatmap' ? 'duplicate-heat-layer' : undefined}
          >
            {image(asset, index)}
            <figcaption>
              <bdi>{asset.name}</bdi>
            </figcaption>
          </figure>
        ))}
        {ready && mode === 'Slider' && (
          <div
            className="duplicate-slider-line"
            style={{ left: `${position}%` }}
            aria-hidden="true"
          />
        )}
      </div>
      {mode === 'Slider' && (
        <Field.Root>
          <Field.Label>Comparison slider · {position}%</Field.Label>
          <Input
            aria-label="Comparison slider"
            type="range"
            min="0"
            max="100"
            value={position}
            disabled={!ready}
            onChange={(event) => setPosition(Number(event.target.value))}
          />
        </Field.Root>
      )}
      {mode === 'Difference heatmap' && (
        <>
          <svg width="0" height="0" aria-hidden="true">
            <filter id={`heat-${id}`} colorInterpolationFilters="sRGB">
              <feColorMatrix type="saturate" values="0" />
              <feComponentTransfer>
                <feFuncR type="table" tableValues="0 0.8 1" />
                <feFuncG type="table" tableValues="0 0.15 1" />
                <feFuncB type="table" tableValues="0 0.05 0.2" />
              </feComponentTransfer>
            </filter>
          </svg>
          <style>{`.duplicate-heat-${id} { filter: url(#heat-${id}); }`}</style>
          <p className="duplicate-preview-note">
            Black: unchanged preview pixels. Warm to pale: increasing difference. Previews are
            unaligned and include colour, scaling and framing differences; this is not an
            original-pixel or damage analysis.
          </p>
        </>
      )}
      {(error || failed.size > 0) && (
        <Button
          onClick={() => {
            setFailed(new Set());
            setError(null);
            previews.retry();
          }}
        >
          Retry comparison previews
        </Button>
      )}
    </section>
  );
}
