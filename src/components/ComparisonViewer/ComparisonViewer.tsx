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
    <dialog
      className="comparison-viewer"
      ref={dialog}
      aria-label="Compare selected photos"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <chakra.header className="viewer-toolbar">
        <Heading as="h2" unstyled>
          Compare {assets.length} selected photos
        </Heading>
        <Button onClick={onClose}>{returnLabel}</Button>
      </chakra.header>
      <Text className="viewer-status">
        Visual comparison does not establish exact duplicates. Only these explicitly selected photos
        are shown. Cached previews are limited to 512 px.
      </Text>
      {previews.error && (
        <Box role="alert">
          {previews.error}
          <Button onClick={previews.retry}>Retry previews</Button>
        </Box>
      )}
      <Box className="comparison-images">
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
    </dialog>
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
