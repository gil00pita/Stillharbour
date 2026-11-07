import { Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef } from 'react';
import type { AssetSummary, LibraryRoot } from '../../bindings';
import { displayDate, fileSize } from '../../library/model';
export function FileDetails({
  asset,
  root,
  onClose,
}: {
  asset: AssetSummary;
  root: LibraryRoot | undefined;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    return () => element?.close();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="file-details"
      aria-labelledby="file-details-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <chakra.header className="section-heading">
        <Heading as="h2" unstyled id="file-details-title">
          File details
        </Heading>
        <Button onClick={onClose}>Close</Button>
      </chakra.header>
      <Heading as="h3" unstyled>
        <bdi>{asset.name}</bdi>
      </Heading>
      <Text className="path">
        <bdi>{asset.relativePath}</bdi>
      </Text>
      <dl>
        <dt>Library</dt>
        <dd>{root?.label ?? 'Unavailable library'}</dd>
        <dt>Size</dt>
        <dd>{fileSize(asset.bytes)}</dd>
        <dt>Modified</dt>
        <dd>{displayDate(asset.modifiedAt ?? '')}</dd>
        <dt>Catalog presence</dt>
        <dd>{asset.presence}</dd>
        <dt>Original library</dt>
        <dd>{root?.availability.replaceAll('_', ' ') ?? 'Unavailable'}</dd>
      </dl>
      <Text className="help">
        This indexed file has no media preview. Browsing does not open external applications or
        change the original.
      </Text>
    </dialog>
  );
}
