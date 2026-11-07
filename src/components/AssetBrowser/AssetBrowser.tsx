import { Box, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import {
  type CSSProperties,
  useEffect,
  useMemo,
  useImperativeHandle,
  useRef,
  type Ref,
  type KeyboardEvent,
  type ReactNode,
} from 'react';
import type { AssetSummary } from '../../bindings';
import {
  DATE_HEADING_HEIGHT,
  THUMBNAIL_SIZES,
  galleryDateSections,
  type DateGrouping,
  type ViewMode,
} from '../../library/model';
import { previewKey, usePreviews } from '../../library/previews';
import { AssetTile } from '@/components/AssetTile';
import { DateHeading } from '@/components/DateHeading';
import { TextListHeader } from '@/components/TextList';
import { DEFAULT_TEXT_COLUMNS, textListLayout, type TextColumn } from '../../library/textColumns';
import { restoreAssetFocus } from '../../library/focus';
import { usePhotoViewport } from '@/hooks/use-photo-viewport';
import type { GalleryDisplay } from '@/components/FilterPanel';
import { galleryPreviewKeys, galleryRenditions } from '../../library/renditionPlan';
import { usePixelRatio } from '@/hooks/use-pixel-ratio';
export type PhotoViewportHandle = {
  remember: (followTop?: boolean) => void;
  scrollBy: (delta: number, mode: number) => void;
  scrollToPeriod: (period: string, progress: number) => boolean;
  scrollToIndex: (index: number) => boolean;
};

interface AssetBrowserProps {
  display?: GalleryDisplay;
  viewportHandle?: Ref<PhotoViewportHandle>;
  onNearStart?: (() => void) | undefined;
  onNearEnd?: (() => void) | undefined;
  pagingPaused?: boolean;
  assets: AssetSummary[];
  startIndex?: number;
  mode: ViewMode;
  selected: ReadonlySet<string>;
  selectionFrozen: boolean;
  activeId: string | null;
  onSelect: (id: string, range: boolean) => void;
  onInspect: (id: string, opener: HTMLButtonElement) => void;
  onFavorite?: ((id: string, favorite: boolean) => Promise<void>) | undefined;
  favoriteBusy?: boolean | undefined;
  onMode?: (mode: ViewMode) => void;
  onError: (error: unknown) => void;
  onVisibleDate: (
    date: string,
    scrolling: boolean,
    progress: number,
    index?: number,
    id?: string,
  ) => void;
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
}

export function AssetBrowser({
  display,
  viewportHandle,
  onNearStart,
  onNearEnd,
  pagingPaused = false,
  assets,
  startIndex = 0,
  mode,
  selected,
  selectionFrozen,
  onSelect,
  onInspect,
  onFavorite,
  favoriteBusy,
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
}: AssetBrowserProps) {
  const size = display?.size ?? 'comfortable';
  const fit = display?.fit ?? 'contain';
  const target = THUMBNAIL_SIZES[size];
  const {
    viewport,
    rows,
    virtual,
    height,
    visibleStart,
    visibleHeight,
    isScrolling,
    remember,
    scrollToAsset,
    scrollToPeriod,
    scrollToIndex,
    isRestorationScroll,
  } = usePhotoViewport({
    assets,
    startIndex,
    mode,
    target,
    grouping,
    onVisibleDate,
    pageKey,
    hasBefore: Boolean(onNearStart),
    hasAfter: Boolean(onNearEnd),
  });
  const offset = visibleStart;
  const bottom = offset + visibleHeight;
  const previewRows = [...virtual].sort(
    (a, b) =>
      Number(b.end > offset && b.start < bottom) - Number(a.end > offset && a.start < bottom),
  );
  const requestRows = isScrolling
    ? previewRows.filter((item) => item.end > offset && item.start < bottom)
    : previewRows;
  const visibleAssets = requestRows.flatMap((item) => rows[item.index]?.assets ?? []);
  const ratio = usePixelRatio();
  const tiers = galleryRenditions(
    previewRows.map((item) => rows[item.index]).filter((row) => row !== undefined),
    ratio,
    mode === 'grid' && fit === 'cover',
  );
  const keys = mode === 'list' ? [] : galleryPreviewKeys(visibleAssets, tiers);
  const previews = usePreviews(keys, onError, !previewPaused, isScrolling ? 32 : undefined);
  const sections = useMemo(() => galleryDateSections(rows), [rows]);
  const previousScroll = useRef(0);
  useImperativeHandle(
    viewportHandle,
    () => ({
      remember,
      scrollToIndex,
      scrollToPeriod: (period, progress) => {
        const found = scrollToPeriod(period, progress);
        if (found && !pagingPaused && progress === 0) onNearStart?.();
        return found;
      },
      scrollBy: (delta, mode) => {
        const element = viewport.current;
        if (!element) return;
        const distance = delta * (mode === 1 ? 16 : mode === 2 ? element.clientHeight : 1);
        if (!pagingPaused && distance < 0 && element.scrollTop < element.clientHeight)
          onNearStart?.();
        element.scrollBy({ top: distance, behavior: 'instant' });
      },
    }),
    [remember, viewport, scrollToIndex, scrollToPeriod, pagingPaused, onNearStart],
  );
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
  function renderRow(item: (typeof virtual)[number], base = 0) {
    const row = rows[item.index];
    if (!row) return null;
    const headingHeight = row.headings.length ? DATE_HEADING_HEIGHT : 0;
    return (
      <Box
        key={item.key}
        className={`asset-row ${mode}`}
        css={{
          position: 'absolute',
          top: '0',
          left: '0',
          width: '100%',
          display: 'flex',
          gap: '6px',
          '&.gallery': {
            gap: '6px',
            '& .asset': { borderRadius: '4px' },
            '& .photo-area': { borderRadius: '4px' },
            '& .asset-open:focus-visible': { outlineOffset: '-3px' },
          },
          '&.list': {
            '& .asset': { width: '100%', borderBottom: '1px solid {colors.border}' },
            '& .text-list-row': { borderRadius: 0, height: '100%' },
            '& .text-list-row .asset-open': {
              display: 'block',
              padding: 0,
              overflow: 'hidden',
              whiteSpace: 'nowrap',
              textOverflow: 'ellipsis',
            },
            '& .text-list-selection .asset-check': {
              position: 'static',
              opacity: 1,
              display: 'block',
              margin: '0 auto',
            },
          },
        }}
        style={{
          transform: `translateY(${item.start - base}px)`,
          height: row.height,
          paddingTop: headingHeight,
        }}
      >
        {row.assets.map((asset, index) => (
          <AssetTile
            key={`${asset.id}:${asset.revision}:${previews.attempt}`}
            asset={asset}
            columns={columns}
            mode={mode}
            width={row.widths[index]}
            imageHeight={row.imageHeight}
            selected={selected.has(asset.id)}
            selectionActive={selected.size > 0 || selectionFrozen}
            frozen={selectionFrozen}
            offline={unavailableRoots.has(asset.rootId)}
            edge={tiers.get(asset.id) ?? 256}
            fit={mode === 'gallery' ? 'contain' : fit}
            tinyPreview={
              previews.results[
                previewKey({ assetId: asset.id, revision: asset.revision, edge: 32 })
              ]
            }
            preview={
              previews.results[
                previewKey({
                  assetId: asset.id,
                  revision: asset.revision,
                  edge: tiers.get(asset.id) ?? 256,
                })
              ]
            }
            onRetry={previews.retry}
            onSelect={onSelect}
            onOpen={onInspect}
            onFavorite={onFavorite}
            favoriteBusy={favoriteBusy}
            onNavigate={navigate}
          />
        ))}
      </Box>
    );
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
    <Box
      className="browser-frame"
      css={{ display: 'flex', flexDirection: 'column', minWidth: '0', minHeight: '0', flex: '1' }}
      style={listStyle}
    >
      {previews.error && (
        <Box className="preview-error" css={{ fontSize: '12px', padding: '8px 0' }} role="status">
          Previews couldn’t be loaded. <Button onClick={previews.retry}>Retry previews</Button>
        </Box>
      )}
      <Box
        className={`browser-canvas-layout ${timeline ? '' : 'without-timeline'}`}
        css={{
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr) 32px',
          gap: '16px',
          flex: '1',
          minHeight: '0',
          '&.without-timeline': { gridTemplateColumns: 'minmax(0, 1fr)' },
          '@media (max-width: 760px)': { gridTemplateColumns: 'minmax(0, 1fr) 32px', gap: '8px' },
        }}
      >
        <BrowserTable mode={mode} columns={columns} sort={sort} onSort={onSort} busy={sortBusy}>
          <Box
            className={`asset-viewport ${mode === 'gallery' ? 'gallery-viewport' : ''} ${timeline ? 'with-timeline' : ''}`}
            css={{
              overflow: 'auto',
              flex: '1',
              minHeight: '0',
              scrollbarGutter: 'stable',
              '&.with-timeline': { scrollbarWidth: 'none', scrollbarGutter: 'auto' },
              '&.with-timeline::-webkit-scrollbar': { display: 'none' },
              '&:focus': { outline: 'none' },
              '&.gallery-viewport': { scrollbarGutter: 'auto' },
              '&.gallery': {
                '& .asset': { borderRadius: '0' },
                '& .photo-area': { borderRadius: '0' },
                '& .photo-area img': { objectFit: 'contain' },
                '& .asset-open:focus-visible': { outlineOffset: '-3px' },
              },
            }}
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
            <Box
              className="virtual-canvas"
              css={{ width: '100%', position: 'relative' }}
              style={{ height }}
            >
              {sections.length
                ? sections.map((section, index) => {
                    const endIndex = sections[index + 1]?.index ?? rows.length;
                    const items = virtual.filter(
                      (item) => item.index >= section.index && item.index < endIndex,
                    );
                    if (!items.length) return null;
                    return (
                      <chakra.section
                        className="photo-date-section"
                        css={{ position: 'absolute', left: '0', width: '100%' }}
                        key={section.key}
                        style={{ top: section.start, height: section.end - section.start }}
                      >
                        <DateHeading
                          heading={section.heading}
                          selected={selected}
                          frozen={selectionFrozen}
                          onSelect={onSelectGroup}
                        />
                        {items.map((item) => renderRow(item, section.start))}
                      </chakra.section>
                    );
                  })
                : virtual.map((item) => renderRow(item))}
            </Box>
          </Box>
        </BrowserTable>
        {timeline}
      </Box>
    </Box>
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
    <Box
      className={`browser-photos ${list ? 'text-list-scroll' : ''}`}
      css={{
        '&.list': {
          '& .asset': { width: '100%', borderBottom: '1px solid {colors.border}' },
          '& .text-list-row': { borderRadius: '0', height: '100%' },
          '& .text-list-row .asset-open': {
            display: 'block',
            padding: '0',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
          },
          '& .text-list-selection .asset-check': {
            position: 'static',
            opacity: '1',
            display: 'block',
            margin: '0 auto',
          },
        },
        display: 'flex',
        flexDirection: 'column',
        minHeight: '0',
        minWidth: '0',
        '&.text-list-scroll': { overflowX: 'auto' },
      }}
    >
      <Box
        className={list ? 'text-list-table' : 'photo-table'}
        css={{
          '&.list': {
            '& .asset': { width: '100%', borderBottom: '1px solid {colors.border}' },
            '& .text-list-row': { borderRadius: '0', height: '100%' },
            '& .text-list-row .asset-open': {
              display: 'block',
              padding: '0',
              overflow: 'hidden',
              whiteSpace: 'nowrap',
              textOverflow: 'ellipsis',
            },
            '& .text-list-selection .asset-check': {
              position: 'static',
              opacity: '1',
              display: 'block',
              margin: '0 auto',
            },
          },
          '&.photo-table': { display: 'flex', flexDirection: 'column', flex: '1', minHeight: '0' },
          '&.text-list-table': {
            display: 'flex',
            flexDirection: 'column',
            flex: '1',
            minHeight: '0',
            minWidth: 'var(--text-list-min-width)',
          },
        }}
        role={list ? 'table' : undefined}
        aria-label={list ? 'Library files' : undefined}
      >
        {list && <TextListHeader columns={columns} sort={sort} onSort={onSort} busy={busy} />}
        {children}
      </Box>
    </Box>
  );
}
