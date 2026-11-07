import { Box, Heading, Text } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useCallback, useState, type RefObject } from 'react';
import { EmptyLibraryArt } from '@/components/EmptyLibraryArt';
import type { LibraryStatus } from '../../bindings';
import type { LibraryController } from '@/hooks/use-library-controller';
import type { ViewMode } from '../../library/model';
import { AssetBrowser, type PhotoViewportHandle } from '@/components/AssetBrowser';
import type { GalleryDisplay } from '@/components/FilterPanel';
import type { TextColumn } from '../../library/textColumns';
import { DateTimeline } from '@/components/DateTimeline';
import { PhotoScanningArt } from '@/components/PhotoScanningArt';
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
  const [visibleDate, setVisibleDate] = useState({
    date: '',
    scrolling: false,
    progress: 0,
    index: 0,
    id: '',
  });
  const onVisibleDate = useCallback(
    (date: string, scrolling: boolean, progress: number, index = 0, id = '') => {
      setVisibleDate((previous) =>
        previous.date === date &&
        previous.scrolling === scrolling &&
        previous.progress === progress &&
        previous.index === index &&
        previous.id === id
          ? previous
          : { date, scrolling, progress, index, id },
      );
    },
    [],
  );
  function scrollTimeline(delta: number, mode: number) {
    viewportHandle.current?.scrollBy(delta, mode);
  }
  return (
    <Box className="collection-layout">
      <Box className="collection-main" data-feed-count={page?.assets.length ?? 0}>
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
            startIndex={page.startIndex ?? 0}
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
                  pageKey={feed.key}
                  index={
                    page.assets.some((asset) => asset.id === visibleDate.id)
                      ? visibleDate.index
                      : (page.startIndex ?? 0)
                  }
                  onIndex={(index) => {
                    if (viewportHandle.current?.scrollToIndex(index)) return true;
                    c.jumpIndex(index);
                    return false;
                  }}
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
            emptyLibrary={
              page?.total === 0 &&
              !feed.error &&
              status.roots.length > 0 &&
              status.roots.every(
                (root) =>
                  root.fileCount === 0 &&
                  root.availability === 'online' &&
                  root.lastScan !== null &&
                  !root.preparingPreviews,
              ) &&
              !status.jobs.some((job) =>
                ['queued', 'running', 'cancel_requested'].includes(job.state),
              )
            }
            preparing={status.roots.some(
              (root) => root.preparingPreviews && (!query.rootId || root.id === query.rootId),
            )}
            scanning={status.jobs.some(
              (job) =>
                (!query.rootId || job.rootId === query.rootId) &&
                job.kind !== 'thumbnails' &&
                ['queued', 'running', 'cancel_requested'].includes(job.state),
            )}
            animating={status.jobs.some(
              (job) =>
                (!query.rootId || job.rootId === query.rootId) &&
                ['queued', 'running'].includes(job.state),
            )}
          />
        )}
        {(feed.busy || feed.error) && (
          <Box className="feed-status" role="status">
            {feed.error ? (
              <>
                <Text as="span">{feed.error.message}</Text>
                <Button onClick={feed.retry}>Retry loading</Button>
              </>
            ) : (
              <Text as="span">Loading photos…</Text>
            )}
          </Box>
        )}
        <Text className="screen-reader-status" aria-live="polite">
          {page?.total.toLocaleString() ?? 0} matching photos and videos
        </Text>
      </Box>
    </Box>
  );
}

function unavailableRoots(status: LibraryStatus) {
  return new Set(
    status.roots
      .filter((root) => !['online', 'scanning'].includes(root.availability))
      .map((root) => root.id),
  );
}
function CollectionEmpty({
  emptyLibrary,
  loading,
  scanning,
  preparing,
  animating,
}: {
  emptyLibrary: boolean;
  loading: boolean;
  scanning: boolean;
  preparing: boolean;
  animating: boolean;
}) {
  const showIllustration = emptyLibrary && !loading && !scanning && !preparing;
  return (
    <Box className="empty-results">
      {(preparing || scanning) && <PhotoScanningArt active={animating} />}
      {showIllustration && <EmptyLibraryArt />}
      <Heading as="h2" unstyled>
        {preparing
          ? 'Preparing your photos'
          : scanning
            ? 'Scanning your photos'
            : loading
              ? 'Reading your catalog…'
              : showIllustration
                ? 'Your library is empty'
                : 'No matching photos yet'}
      </Heading>
      <Text>
        {preparing
          ? 'Photos appear as their previews become ready.'
          : scanning
            ? 'Your scan is running. Photos appear as they are indexed.'
            : showIllustration
              ? 'No photos or videos were found. Check your scan settings or rescan your library folder.'
              : 'Check your filters or select a library to rescan its folder.'}
      </Text>
    </Box>
  );
}
function CollectionTimeline({
  controller: c,
  pageKey,
  index,
  onIndex,
  date,
  scrolling,
  onScroll,
  progress,
  onPosition,
}: {
  controller: LibraryController;
  pageKey: string;
  index: number;
  onIndex: (index: number) => boolean;
  date: string;
  scrolling: boolean;
  onScroll: (delta: number, mode: number) => void;
  progress: number;
  onPosition: (period: string, progress: number) => boolean;
}) {
  if (!['capture', 'added'].includes(c.query.sort) || !c.buckets.length) return null;
  return (
    <DateTimeline
      pageKey={pageKey}
      index={index}
      onIndex={onIndex}
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
