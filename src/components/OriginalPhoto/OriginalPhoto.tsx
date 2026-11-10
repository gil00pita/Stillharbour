import { Box, Text, Button } from '@chakra-ui/react';
import { useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { ORIGINAL_IMAGE_POLICY, type AssetSummary, type ThumbnailResult } from '../../bindings';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { LoadingSheen, ViewerImage } from '@/components/ViewerImage';

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
    <Box
      className={`original-photo ${state === 'loaded' ? 'has-original' : ''} ${state === 'failed' ? 'has-failed' : ''}`}
      css={{
        position: 'absolute',
        inset: '0',
        '&.has-original .original-image': { opacity: '1' },
        '&.has-original .original-fallback': { visibility: 'hidden' },
        '&:not(.has-failed) .viewer-quality': { display: 'none' },
        '&:not(.has-failed) .viewer-loading p': { display: 'none' },
      }}
      aria-busy={state === 'loading'}
    >
      <Box
        className="original-fallback"
        css={{ position: 'absolute', inset: '0', '& .preview-loading-sheen': { display: 'none' } }}
        aria-hidden={state === 'loaded'}
      >
        <ViewerImage
          asset={asset}
          results={results}
          transportError={transportError}
          initialSrc={initialSrc}
          onRetry={onRetry}
        />
      </Box>
      {state !== 'failed' && (
        <CachedPreviewImage
          key={attempt}
          className="original-image"
          css={{
            position: 'absolute',
            inset: '0',
            opacity: '0',
            '@media (prefers-reduced-motion: reduce)': { transition: 'none !important' },
          }}
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
          <Text
            className="original-notice"
            css={{
              position: 'absolute',
              top: '8px',
              left: '8px',
              right: '8px',
              margin: '0',
              padding: '8px',
              background: '{colors.bg.panel}',
              color: '{colors.fg.muted}',
              fontSize: '12px',
            }}
            role="status"
          >
            Loading original…
          </Text>
        </>
      )}
      {state === 'failed' && (
        <Text
          className="original-notice"
          css={{
            position: 'absolute',
            top: '8px',
            left: '8px',
            right: '8px',
            margin: '0',
            padding: '8px',
            background: '{colors.bg.panel}',
            color: '{colors.fg.muted}',
            fontSize: '12px',
          }}
          role="status"
          title="Original viewing supports static JPEG, PNG and WebP, plus HEIC/HEIF on compatible macOS webviews, up to 32 MiB and 32 megapixels. Offline originals must be reconnected."
        >
          Original unavailable · showing a preview.{' '}
          <Button unstyled type="submit" onClick={retryOriginal}>
            Retry original
          </Button>
        </Text>
      )}
    </Box>
  );
}
