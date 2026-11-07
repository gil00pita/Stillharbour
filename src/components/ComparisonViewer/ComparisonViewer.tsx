import { Box, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { useEffect, useRef, useState } from 'react';
import type { AssetSummary, LibraryStatus } from '../../bindings';
import { previewEdges, previewKey, previewUrl, usePreviews } from '../../library/previews';

export function ComparisonViewer({
  assets,
  status,
  onClose,
  onError,
  returnLabel = 'Back to gallery',
}: {
  assets: AssetSummary[];
  status: LibraryStatus;
  onClose: () => void;
  onError: (error: unknown) => void;
  returnLabel?: string;
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
    <chakra.dialog
      className="comparison-viewer"
      css={{
        position: 'fixed',
        inset: '0',
        width: '100vw',
        height: '100vh',
        maxWidth: 'none',
        maxHeight: 'none',
        padding: '0',
        margin: '0',
        border: '0',
        color: 'var(--stillharbour-colors-fg, {colors.fg})',
        background: '{colors.bg.sidebar}',
        '&[open]': { display: 'flex', flexDirection: 'column' },
        '&::backdrop': { background: '{colors.bg.sidebar}' },
      }}
      ref={dialog}
      aria-label="Compare selected photos"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
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
          Compare {assets.length} selected photos
        </Heading>
        <Button onClick={onClose}>{returnLabel}</Button>
      </chakra.header>
      <Text
        className="viewer-status"
        css={{
          color: '{colors.fg.muted}',
          fontSize: '11px',
          margin: '0',
          padding: '10px 20px',
          flexShrink: '0',
        }}
      >
        Visual comparison does not establish exact duplicates. Only these explicitly selected photos
        are shown. Cached previews are limited to 512 px.
      </Text>
      {previews.error && (
        <Box role="alert">
          {previews.error}
          <Button onClick={previews.retry}>Retry previews</Button>
        </Box>
      )}
      <Box
        className="comparison-images"
        css={{
          flex: '1',
          minHeight: '0',
          display: 'grid',
          gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
          gridAutoRows: 'minmax(0, 1fr)',
          gap: '16px',
          padding: '16px',
          '& figure': {
            margin: '0',
            display: 'flex',
            flexDirection: 'column',
            minHeight: '0',
            minWidth: '0',
          },
          '& figure > div': { flex: '1', minHeight: '0', display: 'grid', placeItems: 'center' },
          '& img': { width: '100%', height: '100%', objectFit: 'contain', minHeight: '0' },
          '& figcaption': {
            fontSize: '12px',
            color: '{colors.fg.muted}',
            padding: '10px 0',
            overflowWrap: 'anywhere',
          },
        }}
      >
        {assets.map((asset) => {
          const preview =
            previews.results[
              previewKey({ assetId: asset.id, revision: asset.revision, edge: 512 })
            ];
          const edge = previewEdges([preview])[0];
          return (
            <figure key={asset.id}>
              <Box>
                {edge ? (
                  <ComparisonImage
                    key={`${asset.id}:${asset.revision}:${edge}`}
                    asset={asset}
                    edge={edge}
                  />
                ) : (
                  <Text>{preview?.error ?? 'Preparing cached preview…'}</Text>
                )}
              </Box>
              <figcaption>
                <bdi>{asset.name}</bdi> · Catalog: {asset.presence} · Original library:{' '}
                {status.roots.find((root) => root.id === asset.rootId)?.availability ??
                  'availability unknown'}
              </figcaption>
            </figure>
          );
        })}
      </Box>
    </chakra.dialog>
  );
}

function ComparisonImage({ asset, edge }: { asset: AssetSummary; edge: number }) {
  const [failed, setFailed] = useState(false);
  if (failed)
    return (
      <Text role="status">Cached preview failed to load. The original has not been changed.</Text>
    );
  return (
    <CachedPreviewImage
      src={previewUrl(asset, edge)}
      alt={asset.name}
      onError={() => setFailed(true)}
    />
  );
}
