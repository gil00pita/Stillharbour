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
    <chakra.dialog
      ref={dialog}
      className="file-details"
      css={{
        background: '{colors.bg.panel}',
        color: 'inherit',
        border: '1px solid {colors.border}',
        padding: '22px',
        width: 'min(500px, 92vw)',
        maxHeight: '90vh',
        '&::backdrop': { background: '#0005' },
        '& dl': {
          display: 'grid',
          gridTemplateColumns: 'auto minmax(0, 1fr)',
          gap: '12px 18px',
          fontSize: '12px',
        },
        '& dd': { margin: '0', overflowWrap: 'anywhere' },
        '& h3': { overflowWrap: 'anywhere' },
      }}
      aria-labelledby="file-details-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <chakra.header
        className="section-heading"
        css={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
        }}
      >
        <Heading as="h2" unstyled id="file-details-title">
          File details
        </Heading>
        <Button onClick={onClose}>Close</Button>
      </chakra.header>
      <Heading as="h3" unstyled>
        <bdi>{asset.name}</bdi>
      </Heading>
      <Text
        className="path"
        css={{
          fontSize: '13px',
          lineHeight: '1.6',
          overflowWrap: 'anywhere',
          whiteSpace: 'pre-wrap',
          margin: '8px 0 0',
          color: '{colors.fg.muted}',
        }}
      >
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
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        This indexed file has no media preview. Browsing does not open external applications or
        change the original.
      </Text>
    </chakra.dialog>
  );
}
