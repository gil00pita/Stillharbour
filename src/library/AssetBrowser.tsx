import {
  type CSSProperties,
  useEffect,
  useImperativeHandle,
  useRef,
  type Ref,
  type KeyboardEvent,
  type ReactNode,
} from 'react';
import type { AssetSummary } from '../bindings';
import {
  DATE_HEADING_HEIGHT,
  PHOTO_GUTTER,
  THUMBNAIL_SIZES,
  dayLabel,
  type DateGrouping,
  type ViewMode,
} from './model';
import { previewKey, usePreviews } from './previews';
import { AssetTile } from './AssetTile';
import { TextListHeader } from './TextList';
import { DEFAULT_TEXT_COLUMNS, textListLayout, type TextColumn } from './textColumns';
import { restoreAssetFocus } from './focus';
import { usePhotoViewport } from './usePhotoViewport';
import type { GalleryDisplay } from './FilterPanel';
export type PhotoViewportHandle = { remember: () => void };

export function AssetBrowser({
  display,
  viewportHandle,
  onNearStart,
  onNearEnd,
  pagingPaused = false,
  assets,
  mode,
  selected,
  selectionFrozen,
  onSelect,
  onInspect,
  onError,
  onVisibleDate,
  pageKey,
  onSelectGroup,
  onClearSelection,
  timeline,
  previewPaused = false,
  unavailableRoots = new Set<string>(),
  grouping = 'capture',
  columns = DEFAULT_TEXT_COLUMNS,
  sort,
  onSort,
  sortBusy = false,
}: {
  display?: GalleryDisplay;
  viewportHandle?: Ref<PhotoViewportHandle>;
  onNearStart?: (() => void) | undefined;
  onNearEnd?: (() => void) | undefined;
  pagingPaused?: boolean;
  assets: AssetSummary[];
  mode: ViewMode;
  selected: ReadonlySet<string>;
  selectionFrozen: boolean;
  activeId: string | null;
  onSelect: (id: string, range: boolean) => void;
  onInspect: (id: string, opener: HTMLButtonElement) => void;
  onMode?: (mode: ViewMode) => void;
  onError: (error: unknown) => void;
  onVisibleDate: (date: string, scrolling: boolean) => void;
  pageKey: string;
  onSelectGroup?: (ids: string[]) => void;
  onClearSelection?: () => void;
  previewPaused?: boolean;
  unavailableRoots?: ReadonlySet<string>;
  grouping?: DateGrouping;
  timeline?: ReactNode;
  columns?: readonly TextColumn[];
  sort?: string;
  onSort?: (sort: string) => void;
  sortBusy?: boolean;
}) {
  const size = display?.size ?? 'comfortable';
  const fit = display?.fit ?? 'contain';
  const target = THUMBNAIL_SIZES[size];
  const { viewport, rows, virtual, height, remember, scrollToAsset, isRestorationScroll } =
    usePhotoViewport({
      assets,
      mode,
      target,
      grouping,
      onVisibleDate,
      pageKey,
    });
  const visibleAssets = virtual.flatMap((item) => rows[item.index]?.assets ?? []);
  const edge = target >= 220 ? 512 : 256;
  const keys =
    mode === 'list'
      ? []
      : visibleAssets
          .slice(0, 240)
          .map((asset) => ({ assetId: asset.id, revision: asset.revision, edge }));
  const previews = usePreviews(keys, onError, !previewPaused);
  const previousScroll = useRef(0);
  useImperativeHandle(viewportHandle, () => ({ remember }), [remember]);
  function endEdge() {
    const element = viewport.current;
    if (
      element &&
      !pagingPaused &&
      element.scrollHeight - element.scrollTop - element.clientHeight < element.clientHeight
    )
      onNearEnd?.();
  }
  useEffect(() => {
    endEdge();
  });
  function navigate(event: KeyboardEvent<HTMLElement>) {
    if (event.key === 'Escape') {
      onClearSelection?.();
      return;
    }
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement)
      return;
    const article = (event.target as HTMLElement).closest<HTMLElement>('[data-asset-id]');
    if (
      !pagingPaused &&
      ['PageUp', 'Home'].includes(event.key) &&
      (viewport.current?.scrollTop ?? 0) < (viewport.current?.clientHeight ?? 0)
    )
      onNearStart?.();
    const index = assets.findIndex((asset) => asset.id === article?.dataset.assetId);
    if (index < 0) return;
    if (index === 0 && event.key === 'ArrowUp' && !pagingPaused) onNearStart?.();
    let next = index;
    if (event.key === 'ArrowRight') next++;
    else if (event.key === 'ArrowLeft') next--;
    else if (event.key === 'Home') next = 0;
    else if (event.key === 'End') next = assets.length - 1;
    else if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      const rowIndex = rows.findIndex((row) =>
        row.assets.some((asset) => asset.id === article?.dataset.assetId),
      );
      const row = rows[rowIndex];
      const destination = rows[rowIndex + (event.key === 'ArrowUp' ? -1 : 1)];
      const column = row?.assets.findIndex((asset) => asset.id === article?.dataset.assetId) ?? 0;
      const asset = destination?.assets[Math.min(column, destination.assets.length - 1)];
      next = asset ? assets.findIndex((item) => item.id === asset.id) : index;
    } else return;
    event.preventDefault();
    const asset = assets[Math.max(0, Math.min(assets.length - 1, next))];
    if (!asset) return;
    if (event.shiftKey && !selectionFrozen) onSelect(asset.id, true);
    scrollToAsset(asset.id);
    restoreAssetFocus(asset.id, null);
  }
  const listLayout = textListLayout(columns);
  const listStyle =
    mode === 'list'
      ? ({
          '--text-list-grid': listLayout.grid,
          '--text-list-min-width': `${listLayout.minWidth}px`,
        } as CSSProperties)
      : undefined;
  return (
    <div className="browser-frame" style={listStyle}>
      {previews.error && (
        <div className="preview-error" role="status">
          Previews couldn’t be loaded. <button onClick={previews.retry}>Retry previews</button>
        </div>
      )}
      <div className={`browser-canvas-layout ${timeline ? '' : 'without-timeline'}`}>
        <BrowserTable mode={mode} columns={columns} sort={sort} onSort={onSort} busy={sortBusy}>
          <div
            className={`asset-viewport ${mode === 'gallery' ? 'gallery-viewport' : ''}`}
            ref={viewport}
            onScroll={() => {
              const element = viewport.current;
              if (!element) return;
              const upward = element.scrollTop < previousScroll.current;
              previousScroll.current = element.scrollTop;
              if (isRestorationScroll()) return;
              if (!pagingPaused && upward && element.scrollTop < element.clientHeight)
                onNearStart?.();
              endEdge();
            }}
            onWheel={(event) => {
              const element = viewport.current;
              if (
                !pagingPaused &&
                element &&
                event.deltaY < 0 &&
                element.scrollTop < element.clientHeight
              )
                onNearStart?.();
            }}
            role={mode === 'list' ? 'rowgroup' : undefined}
            data-view-mode={mode}
            data-thumbnail-size={size}
            data-grid-fit={fit}
            tabIndex={0}
            aria-label={mode === 'list' ? 'Text-only library' : 'Photo library'}
          >
            <div className="virtual-canvas" style={{ height }}>
              {virtual.map((item) => {
                const row = rows[item.index];
                if (!row) return null;
                const headingHeight = row.headings.length ? DATE_HEADING_HEIGHT : 0;
                return (
                  <div
                    key={item.key}
                    className={`asset-row ${mode}`}
                    style={{
                      transform: `translateY(${item.start}px)`,
                      height: row.height,
                      paddingTop: headingHeight,
                    }}
                  >
                    {row.headings.map((heading) => (
                      <div
                        key={heading.date}
                        className="date-heading"
                        data-day={heading.date}
                        style={{
                          left:
                            row.widths.slice(0, heading.index).reduce((a, b) => a + b, 0) +
                            PHOTO_GUTTER * heading.index,
                          width: heading.width,
                        }}
                      >
                        <input
                          type="checkbox"
                          aria-label={`Select ${heading.ids.length} shown photos from ${dayLabel(heading.date)}`}
                          checked={
                            heading.ids.length > 0 && heading.ids.every((id) => selected.has(id))
                          }
                          disabled={selectionFrozen}
                          onChange={() => onSelectGroup?.(heading.ids)}
                        />
                        <h2 title={dayLabel(heading.date)} aria-label={dayLabel(heading.date)}>
                          {dayLabel(heading.date, undefined, heading.width < 250)}
                        </h2>
                        {heading.width >= 180 && <small>{heading.ids.length} shown</small>}
                      </div>
                    ))}
                    {row.assets.map((asset, index) => (
                      <AssetTile
                        key={`${asset.id}:${asset.revision}`}
                        asset={asset}
                        columns={columns}
                        mode={mode}
                        width={row.widths[index]}
                        imageHeight={row.imageHeight}
                        selected={selected.has(asset.id)}
                        selectionActive={selected.size > 0 || selectionFrozen}
                        frozen={selectionFrozen}
                        offline={unavailableRoots.has(asset.rootId)}
                        edge={edge}
                        fit={mode === 'gallery' ? 'contain' : fit}
                        preview={
                          previews.results[
                            previewKey({ assetId: asset.id, revision: asset.revision, edge })
                          ]
                        }
                        onSelect={onSelect}
                        onOpen={onInspect}
                        onNavigate={navigate}
                      />
                    ))}
                  </div>
                );
              })}
            </div>
          </div>
        </BrowserTable>
        {timeline}
      </div>
    </div>
  );
}

function BrowserTable({
  mode,
  columns,
  sort,
  onSort,
  busy,
  children,
}: {
  mode: ViewMode;
  columns: readonly TextColumn[];
  sort: string | undefined;
  onSort: ((sort: string) => void) | undefined;
  busy: boolean;
  children: ReactNode;
}) {
  const list = mode === 'list';
  return (
    <div className={`browser-photos ${list ? 'text-list-scroll' : ''}`}>
      <div
        className={list ? 'text-list-table' : 'photo-table'}
        role={list ? 'table' : undefined}
        aria-label={list ? 'Library files' : undefined}
      >
        {list && <TextListHeader columns={columns} sort={sort} onSort={onSort} busy={busy} />}
        {children}
      </div>
    </div>
  );
}
