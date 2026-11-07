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
      <Box className="viewer-loading">
        <LoadingSheen />
        <Text role="status">Preparing cached preview…</Text>
      </Box>
    ) : (
      <Box className="viewer-unavailable" role="status">
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
          <Text className="viewer-quality" role="status">
            Loading photo preview…
          </Text>
        </>
      ) : (
        !revealed && (
          <Text className="viewer-quality" role="status">
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
    <Text as="span" className="preview-loading-sheen" aria-hidden="true" data-paused={hidden} />
  );
}
