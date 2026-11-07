import { Box, Text, chakra, Button } from '@chakra-ui/react';
import { useEffect, useMemo, useState, type RefObject } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { DirectoryEntry, LibraryRoot } from '../../bindings';
import { TileImage } from '@/components/AssetTile';
import { fileSize } from '../../library/model';
import { previewKey, usePreviews } from '../../library/previews';
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
        <Text role="status">
          Previews could not be loaded.{' '}
          <Button unstyled type="submit" onClick={previews.retry}>
            Retry previews
          </Button>
        </Text>
      )}
      <Box
        ref={viewport}
        className={`asset-viewport explorer-viewport ${mode}`}
        css={{
          overflow: 'auto',
          flex: '1',
          minHeight: '0',
          scrollbarGutter: 'stable',
          '&.with-timeline': { scrollbarWidth: 'none', scrollbarGutter: 'auto' },
          '&.with-timeline::-webkit-scrollbar': { display: 'none' },
          '&:focus': { outline: 'none' },
          minWidth: '0',
          '&.details .explorer-entry': {
            flexDirection: 'row',
            alignItems: 'center',
            gap: '18px',
            borderBottomColor: '{colors.border}',
            borderRadius: '0',
          },
          '&.details .explorer-name': { flex: '1' },
          '&.details .explorer-kind': { width: '40%', textAlign: 'right' },
        }}
        tabIndex={0}
        aria-label="Indexed folder contents"
      >
        <Box
          className="virtual-canvas"
          css={{ width: '100%', position: 'relative' }}
          style={{ height: virtualizer.getTotalSize() }}
        >
          {visible.map((row) => (
            <Box
              key={row.key}
              className="explorer-row"
              css={{
                position: 'absolute',
                top: '0',
                left: '0',
                width: '100%',
                display: 'grid',
                gap: '12px',
                paddingBottom: '12px',
              }}
              style={{
                transform: `translateY(${row.start}px)`,
                height: row.size,
                gridTemplateColumns: `repeat(${columns},minmax(0,1fr))`,
              }}
            >
              {rows[row.index]?.map((entry) => (
                <chakra.article
                  key={itemId(entry)}
                  data-asset-id={itemId(entry)}
                  className="explorer-item"
                  css={{ minWidth: '0', minHeight: '0' }}
                >
                  <Button
                    unstyled
                    type="submit"
                    className="asset-open explorer-entry"
                    css={{
                      border: '0',
                      background: 'transparent',
                      padding: '0',
                      color: 'inherit',
                      width: '100%',
                      textAlign: 'left',
                      fontSize: '12px',
                      height: '100%',
                      display: 'flex',
                      flexDirection: 'column',
                      'button&.asset-open': {
                        display: 'flex',
                        flexDirection: 'column',
                        width: '100%',
                        height: '100%',
                        padding: '8px',
                        gap: '6px',
                        border: '1px solid transparent',
                        borderRadius: '4px',
                        textAlign: 'left',
                        color: 'inherit',
                        background: 'transparent',
                      },
                      'button&.asset-open:hover:enabled': {
                        background: '{colors.bg.panel}',
                        borderColor: '{colors.border}',
                      },
                      '& .photo-area': { width: '100%', flexShrink: '0' },
                    }}
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
                        <Text
                          as="span"
                          className="explorer-glyph"
                          css={{
                            height: '132px',
                            width: '100%',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            color: '{colors.brand.solid}',
                          }}
                        >
                          <FolderGlyph file={entry.kind === 'file'} />
                        </Text>
                      ))}
                    <Text
                      as="span"
                      className="explorer-name"
                      css={{
                        fontSize: '12px',
                        width: '100%',
                        overflow: 'hidden',
                        whiteSpace: 'nowrap',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      <bdi>{itemName(entry)}</bdi>
                    </Text>
                    <Text
                      as="span"
                      className="explorer-kind"
                      css={{
                        color: '{colors.fg.muted}',
                        fontSize: '11px',
                        width: '100%',
                        overflow: 'hidden',
                        whiteSpace: 'nowrap',
                        textOverflow: 'ellipsis',
                      }}
                    >
                      {itemStatus(entry)}
                    </Text>
                  </Button>
                </chakra.article>
              ))}
            </Box>
          ))}
        </Box>
      </Box>
    </>
  );
}
