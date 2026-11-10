import { Box, Heading, Text, Button } from '@chakra-ui/react';
import { useState } from 'react';
import { useDocumentHidden } from '@/hooks/use-document-hidden';
import { VIEWER_EDGE, type AssetSummary, type ThumbnailResult } from '../../bindings';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { previewEdges, previewUrl } from '../../library/previews';

export function ViewerImage({
  asset,
  results,
  transportError,
  onRetry,
  initialSrc,
}: {
  asset: AssetSummary;
  results: (ThumbnailResult | undefined)[];
  transportError: string | null;
  onRetry: () => void;
  initialSrc?: string | undefined;
}) {
  const [failed, setFailed] = useState<ReadonlySet<number>>(new Set());
  const [loaded, setLoaded] = useState<ReadonlySet<number>>(new Set());
  const [attempt, setAttempt] = useState(0);
  const [originFailed, setOriginFailed] = useState(false);
  const available = previewEdges(results, failed);
  const high = available.includes(VIEWER_EDGE);
  const small = available.find((edge) => edge < VIEWER_EDGE);
  const revealed = high && loaded.has(VIEWER_EDGE);
  const pending = !transportError && results.some((item) => !item || item.state === 'pending');
  const loading =
    !revealed &&
    (pending || (high && !failed.has(VIEWER_EDGE)) || (small !== undefined && !loaded.has(small)));
  const error =
    transportError ??
    results.find((item) => item?.error)?.error ??
    'Cached preview failed to load.';
  const origin = !originFailed && initialSrc;
  function fail(edge: number) {
    setFailed((previous) => new Set([...previous, edge]));
  }
  function didLoad(edge: number) {
    setLoaded((previous) => new Set([...previous, edge]));
  }
  function retry() {
    setFailed(new Set());
    setLoaded(new Set());
    setAttempt((value) => value + 1);
    setOriginFailed(false);
    onRetry();
  }
  function url(edge: number) {
    return `${previewUrl(asset, edge)}${attempt ? `?request=${attempt}` : ''}`;
  }
  if (!high && small === undefined && !origin) {
    return loading ? (
      <Box
        className="viewer-loading"
        css={{
          display: 'grid',
          placeContent: 'center',
          textAlign: 'center',
          minHeight: '100%',
          margin: '0',
          padding: '32px',
          color: '{colors.fg.muted}',
          fontSize: '13px',
          position: 'relative',
          overflow: 'hidden',
          background: '{colors.bg.panel}',
        }}
      >
        <LoadingSheen />
        <Text role="status">Preparing cached preview…</Text>
      </Box>
    ) : (
      <Box
        className="viewer-unavailable"
        css={{
          display: 'grid',
          placeContent: 'center',
          textAlign: 'center',
          minHeight: '100%',
          margin: '0',
          padding: '32px',
          color: '{colors.fg.muted}',
          fontSize: '13px',
        }}
        role="status"
      >
        <Heading as="h3" unstyled>
          <bdi>{asset.name}</bdi>
        </Heading>
        <Text>{error}</Text>
        <Text>The original has not been changed.</Text>
        <Button unstyled type="submit" onClick={retry}>
          Retry preview
        </Button>
      </Box>
    );
  }
  return (
    <Box
      className={`viewer-renditions ${revealed ? 'is-revealed' : ''} ${document.visibilityState === 'hidden' ? 'without-motion' : ''}`}
      css={{
        position: 'relative',
        width: '100%',
        height: '100%',
        '& img': { position: 'absolute', inset: '0' },
        '&.is-revealed .viewer-rendition-high': { opacity: '1', filter: 'blur(0)' },
        '&.is-revealed .viewer-rendition-small': { opacity: '0' },
        '&.without-motion .viewer-rendition-high': { transition: 'none', filter: 'none' },
        '&.without-motion .viewer-rendition-small': { transition: 'none', filter: 'none' },
      }}
      aria-busy={loading}
    >
      {origin && !revealed && (small === undefined || !loaded.has(small)) && (
        <CachedPreviewImage
          className="viewer-rendition-origin"
          src={origin}
          alt=""
          draggable={false}
          onError={() => setOriginFailed(true)}
        />
      )}
      {small !== undefined && (
        <CachedPreviewImage
          key={`${small}:${attempt}`}
          className="viewer-rendition-small"
          css={{
            transition: 'opacity 400ms ease',
            '@media (prefers-reduced-motion: reduce)': { transition: 'none' },
          }}
          src={url(small)}
          alt={asset.name}
          draggable={false}
          onLoad={() => didLoad(small)}
          onError={() => fail(small)}
        />
      )}
      {high && (
        <CachedPreviewImage
          key={`${VIEWER_EDGE}:${attempt}`}
          className="viewer-rendition-high"
          css={{
            opacity: '0',
            filter: 'blur(2px)',
            transition: 'opacity 400ms ease,\n    filter 400ms ease',
            '@media (prefers-reduced-motion: reduce)': { transition: 'none', filter: 'none' },
          }}
          src={url(VIEWER_EDGE)}
          alt={small !== undefined ? '' : asset.name}
          draggable={false}
          onLoad={() => didLoad(VIEWER_EDGE)}
          onError={() => fail(VIEWER_EDGE)}
        />
      )}
      {loading ? (
        <>
          <LoadingSheen />
          <Text
            className="viewer-quality"
            css={{
              position: 'absolute',
              bottom: '8px',
              left: '8px',
              right: '8px',
              fontSize: '12px',
              background: '{colors.bg.panel}',
              padding: '8px',
            }}
            role="status"
          >
            Loading photo preview…
          </Text>
        </>
      ) : (
        !revealed && (
          <Text
            className="viewer-quality"
            css={{
              position: 'absolute',
              bottom: '8px',
              left: '8px',
              right: '8px',
              fontSize: '12px',
              background: '{colors.bg.panel}',
              padding: '8px',
            }}
            role="status"
          >
            Smaller cached preview ·{' '}
            {failed.has(VIEWER_EDGE) ? 'Higher detail could not be loaded.' : error}{' '}
            <Button unstyled type="submit" onClick={retry}>
              Retry higher detail
            </Button>
          </Text>
        )
      )}
    </Box>
  );
}

export function LoadingSheen() {
  const hidden = useDocumentHidden();
  return (
    <Text
      as="span"
      className="preview-loading-sheen"
      css={{
        position: 'absolute',
        inset: '0',
        overflow: 'hidden',
        pointerEvents: 'none',
        '&::after': {
          content: "''",
          position: 'absolute',
          inset: '0',
          background:
            'linear-gradient(\n    110deg,\n    transparent 25%,\n    rgb(255 255 255 / 0.04) 40%,\n    rgb(255 255 255 / 0.22) 50%,\n    rgb(255 255 255 / 0.04) 60%,\n    transparent 75%\n  )',
          transform: 'translateX(-110%)',
          animation: 'preview-loading-glare 1800ms linear infinite',
        },
        "&[data-paused='true']::after": { animationPlayState: 'paused' },
        '@media (prefers-reduced-motion: reduce)': {
          '&::after': { animation: 'none', transform: 'none', opacity: '0.3' },
        },
      }}
      aria-hidden="true"
      data-paused={hidden}
    />
  );
}
