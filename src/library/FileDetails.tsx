import { useEffect, useRef } from 'react';
import type { AssetSummary, LibraryRoot } from '../bindings';
import { displayDate, fileSize } from './model';
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
      <header className="section-heading">
        <h2 id="file-details-title">File details</h2>
        <button onClick={onClose}>Close</button>
      </header>
      <h3>
        <bdi>{asset.name}</bdi>
      </h3>
      <p className="path">
        <bdi>{asset.relativePath}</bdi>
      </p>
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
      <p className="help">
        This indexed file has no media preview. Browsing does not open external applications or
        change the original.
      </p>
    </dialog>
  );
}
