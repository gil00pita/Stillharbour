import { useEffect, useMemo, useState, type RefObject } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { DirectoryEntry, LibraryRoot } from '../bindings';
import { TileImage } from './AssetTile';
import { fileSize } from './model';
import { previewKey, usePreviews } from './previews';
export type ExplorerItem = DirectoryEntry | { kind: 'root'; root: LibraryRoot };
function itemId(item: ExplorerItem) {
  return item.kind === 'root'
    ? item.root.id
    : item.kind === 'folder'
      ? item.directory.id
      : item.asset.id;
}
function itemName(item: ExplorerItem) {
  return item.kind === 'root'
    ? item.root.label
    : item.kind === 'folder'
      ? item.directory.name
      : item.asset.name;
}
function itemStatus(item: ExplorerItem) {
  if (item.kind === 'root')
    return `${item.root.fileCount.toLocaleString()} media · ${item.root.availability.replaceAll('_', ' ')}`;
  if (item.kind === 'folder')
    return item.directory.state === 'complete'
      ? 'Folder'
      : item.directory.state === 'blocked'
        ? 'Unavailable entry'
        : item.directory.state === 'retained'
          ? 'Previous scan'
          : 'Partially indexed';
  return `${fileSize(item.asset.bytes)} · ${item.media ? 'Photo / media' : 'File'}${item.asset.presence !== 'present' ? ` · ${item.asset.presence}` : ''}`;
}
export function FolderGlyph({ file = false }: { file?: boolean }) {
  return (
    <svg width="42" height="42" viewBox="0 0 48 48" fill="none" aria-hidden="true">
      <path
        d={file ? 'M12 5h17l9 9v29H12ZM29 5v10h9' : 'M5 14h15l5 5h18v23H5ZM5 14V8h15l5 6'}
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
    </svg>
  );
}
export function ExplorerCanvas({
  items,
  mode,
  viewport,
  root,
  paused,
  onOpen,
  onError,
}: {
  items: ExplorerItem[];
  mode: 'icons' | 'details';
  viewport: RefObject<HTMLDivElement | null>;
  root: LibraryRoot | undefined;
  paused: boolean;
  onOpen: (entry: ExplorerItem, button: HTMLButtonElement) => void;
  onError: (error: unknown) => void;
}) {
  const [width, setWidth] = useState(700);
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setWidth(entries[0]?.contentRect.width ?? 700),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [viewport]);
  const columns = mode === 'details' ? 1 : Math.max(1, Math.floor(width / 180));
  const rows = useMemo(
    () =>
      Array.from({ length: Math.ceil(items.length / columns) }, (_, index) =>
        items.slice(index * columns, (index + 1) * columns),
      ),
    [items, columns],
  );
  // Mutable virtualizer remains at the rendering boundary; no React Compiler.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => viewport.current,
    estimateSize: () => (mode === 'details' ? 64 : 210),
    overscan: 2,
    getItemKey: (index) => itemId(rows[index]![0]!),
  });
  const visible = virtualizer.getVirtualItems();
  const keys =
    mode === 'details'
      ? []
      : visible
          .flatMap((row) => rows[row.index] ?? [])
          .filter(
            (entry): entry is Extract<DirectoryEntry, { kind: 'file' }> =>
              entry.kind === 'file' && entry.media,
          )
          .slice(0, 240)
          .flatMap((entry) =>
            [32, 256].map((edge) => ({
              assetId: entry.asset.id,
              revision: entry.asset.revision,
              edge,
            })),
          );
  const previews = usePreviews(keys, onError, !paused);
  useEffect(() => {
    virtualizer.measure();
  }, [mode, columns, virtualizer]);
  return (
    <>
      {previews.error && (
        <p role="status">
          Previews could not be loaded. <button onClick={previews.retry}>Retry previews</button>
        </p>
      )}
      <div
        ref={viewport}
        className={`asset-viewport explorer-viewport ${mode}`}
        tabIndex={0}
        aria-label="Indexed folder contents"
      >
        <div className="virtual-canvas" style={{ height: virtualizer.getTotalSize() }}>
          {visible.map((row) => (
            <div
              key={row.key}
              className="explorer-row"
              style={{
                transform: `translateY(${row.start}px)`,
                height: row.size,
                gridTemplateColumns: `repeat(${columns},minmax(0,1fr))`,
              }}
            >
              {rows[row.index]?.map((entry) => (
                <article
                  key={itemId(entry)}
                  data-asset-id={itemId(entry)}
                  className="explorer-item"
                >
                  <button
                    className="asset-open explorer-entry"
                    aria-label={`${entry.kind === 'file' ? 'Open' : 'Open folder'} ${itemName(entry)}`}
                    title={itemName(entry)}
                    disabled={entry.kind === 'folder' && entry.directory.state === 'blocked'}
                    onClick={(event) => onOpen(entry, event.currentTarget)}
                  >
                    {mode === 'icons' &&
                      (entry.kind === 'file' && entry.media ? (
                        <TileImage
                          key={`${entry.asset.revision}:256`}
                          tinyPreview={
                            previews.results[
                              previewKey({
                                assetId: entry.asset.id,
                                revision: entry.asset.revision,
                                edge: 32,
                              })
                            ]
                          }
                          asset={entry.asset}
                          imageHeight={132}
                          edge={256}
                          fit="contain"
                          offline={Boolean(
                            root && !['online', 'scanning'].includes(root.availability),
                          )}
                          preview={
                            previews.results[
                              previewKey({
                                assetId: entry.asset.id,
                                revision: entry.asset.revision,
                                edge: 256,
                              })
                            ]
                          }
                        />
                      ) : (
                        <span className="explorer-glyph">
                          <FolderGlyph file={entry.kind === 'file'} />
                        </span>
                      ))}
                    <span className="explorer-name">
                      <bdi>{itemName(entry)}</bdi>
                    </span>
                    <span className="explorer-kind">{itemStatus(entry)}</span>
                  </button>
                </article>
              ))}
            </div>
          ))}
        </div>
      </div>
    </>
  );
}
