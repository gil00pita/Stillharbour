import { useCallback, useState, type RefObject } from 'react';
import type { LibraryStatus } from '../bindings';
import type { LibraryController } from './useLibraryController';
import type { ViewMode } from './model';
import { AssetBrowser, type PhotoViewportHandle } from './AssetBrowser';
import type { GalleryDisplay } from './FilterPanel';
import type { TextColumn } from './textColumns';
import { DateTimeline } from './DateTimeline';
export function PhotoCollection({
  display,
  viewportHandle,
  controller: c,
  status,
  mode,
  viewerOpen,
  onOpen,
  columns,
}: {
  display: GalleryDisplay;
  columns: readonly TextColumn[];
  viewportHandle: RefObject<PhotoViewportHandle | null>;
  controller: LibraryController;
  status: LibraryStatus;
  mode: ViewMode;
  viewerOpen: boolean;
  onOpen: (id: string, opener: HTMLButtonElement) => void;
}) {
  const { query, page, loading, feed } = c;
  const { ids: selected, frozen, busy: batchBusy } = c.selection;
  const [visibleDate, setVisibleDate] = useState({ date: '', scrolling: false, progress: 0 });
  const onVisibleDate = useCallback((date: string, scrolling: boolean, progress: number) => {
    setVisibleDate((previous) =>
      previous.date === date && previous.scrolling === scrolling && previous.progress === progress
        ? previous
        : { date, scrolling, progress },
    );
  }, []);
  function scrollTimeline(delta: number, mode: number) {
    viewportHandle.current?.scrollBy(delta, mode);
  }
  return (
    <div className="collection-layout">
      <div className="collection-main" data-feed-count={page?.assets.length ?? 0}>
        {page?.assets.length ? (
          <AssetBrowser
            display={display}
            columns={columns}
            sort={query.sort}
            onSort={(sort) => c.filter({ sort })}
            sortBusy={loading || batchBusy}
            viewportHandle={viewportHandle}
            pagingPaused={viewerOpen || feed.busy || Boolean(feed.error)}
            onNearStart={feed.hasBefore ? feed.previous : undefined}
            onNearEnd={feed.hasAfter ? feed.next : undefined}
            assets={page.assets}
            mode={mode}
            selected={selected}
            selectionFrozen={frozen || batchBusy}
            activeId={null}
            onSelect={c.select}
            onInspect={onOpen}
            onError={c.onError}
            onVisibleDate={onVisibleDate}
            pageKey={feed.key}
            timeline={
              c.buckets.length && ['capture', 'added'].includes(query.sort) ? (
                <CollectionTimeline
                  controller={c}
                  onScroll={scrollTimeline}
                  progress={visibleDate.progress}
                  onPosition={(period, progress) =>
                    viewportHandle.current?.scrollToPeriod(period, progress) ?? false
                  }
                  date={visibleDate.date}
                  scrolling={visibleDate.scrolling}
                />
              ) : null
            }
            previewPaused={viewerOpen}
            grouping={
              query.sort === 'capture' ? 'capture' : query.sort === 'added' ? 'added' : null
            }
            unavailableRoots={unavailableRoots(status)}
            onClearSelection={c.clear}
            onSelectGroup={c.selectGroup}
          />
        ) : (
          <CollectionEmpty
            loading={loading}
            scanning={status.jobs.some(
              (job) => job.kind !== 'thumbnails' && job.state === 'running',
            )}
          />
        )}
        {(feed.busy || feed.error) && (
          <div className="feed-status" role="status">
            {feed.error ? (
              <>
                <span>{feed.error.message}</span>
                <button onClick={feed.retry}>Retry loading</button>
              </>
            ) : (
              <span>Loading photos…</span>
            )}
          </div>
        )}
        <p className="screen-reader-status" aria-live="polite">
          {page?.total.toLocaleString() ?? 0} matching photos and videos
        </p>
      </div>
    </div>
  );
}

function unavailableRoots(status: LibraryStatus) {
  return new Set(
    status.roots
      .filter((root) => !['online', 'scanning'].includes(root.availability))
      .map((root) => root.id),
  );
}
function CollectionEmpty({ loading, scanning }: { loading: boolean; scanning: boolean }) {
  return (
    <div className="empty-results">
      <h2>{loading ? 'Reading your catalog…' : 'No matching photos yet'}</h2>
      <p>
        {scanning
          ? 'Your scan is running. Photos appear as they are indexed.'
          : 'Check your filters or select a library to rescan its folder.'}
      </p>
    </div>
  );
}
function CollectionTimeline({
  controller: c,
  date,
  scrolling,
  onScroll,
  progress,
  onPosition,
}: {
  controller: LibraryController;
  date: string;
  scrolling: boolean;
  onScroll: (delta: number, mode: number) => void;
  progress: number;
  onPosition: (period: string, progress: number) => boolean;
}) {
  if (!['capture', 'added'].includes(c.query.sort) || !c.buckets.length) return null;
  return (
    <DateTimeline
      progress={progress}
      onPosition={onPosition}
      buckets={c.buckets}
      date={date}
      scrolling={scrolling}
      onYear={c.jump}
      onScroll={onScroll}
    />
  );
}
