import { useCallback, useRef, useState, type ReactNode } from 'react';
import type { AssetSummary, AssetQuery, LibraryStatus } from '../bindings';
import { FilterPanel, type GalleryDisplay } from './FilterPanel';
import type { PhotoViewportHandle } from './AssetBrowser';
import { PhotoCollection } from './PhotoCollection';
import { useLibraryController } from './useLibraryController';
import { LibraryHeader } from './LibraryHeader';
import { PhotoViewer } from './PhotoViewer';
import { ComparisonViewer } from './ComparisonViewer';
import { OrganizationPreview } from './OrganizationPreview';
import { SavedOrganizationPlans } from './SavedOrganizationPlans';
import { BatchToolbar } from './BatchToolbar';
import { TextListControls } from './TextList';
import { PreparationBanner } from './PreparationBanner';
import { DEFAULT_TEXT_COLUMNS } from './textColumns';
import { photoOrigin, type PhotoOrigin } from './viewerMotion';
import { restoreAssetFocus } from './focus';
import { type ViewMode, type Section } from './model';

export type { Section } from './model';
export function LibraryScreen({
  jobsControl,
  navigationControl,
  defaults,
  status,
  section,
  onError,
  onRefresh,
  onSettings,
  onRelink,
}: {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  defaults: import('../bindings').AppPreferences;
  status: LibraryStatus;
  section: Section;
  onError: (error: unknown) => void;
  onRefresh: () => Promise<void>;
  onSettings: (id: string) => void;
  onRelink: (id: string) => Promise<void>;
}) {
  const viewportHandle = useRef<PhotoViewportHandle>(null);
  const rememberWindow = useCallback(
    (followTop = false) => viewportHandle.current?.remember(followTop),
    [],
  );
  const c = useLibraryController({
    status,
    section,
    onError,
    onRefresh,
    defaults,
    onWindowChange: rememberWindow,
  });
  const { query, page, loading, queryError, notice } = c;
  const { ids: selected, frozen, busy: batchBusy, count: selectionCount } = c.selection;
  const [columns, setColumns] = useState([...DEFAULT_TEXT_COLUMNS]);
  const [filterOpen, setFilterOpen] = useState(false);
  const [displayOverride, setDisplay] = useState<Partial<GalleryDisplay>>({});
  const display: GalleryDisplay = {
    size: displayOverride.size ?? defaults.thumbnailSize,
    fit: displayOverride.fit ?? defaults.gridFit,
  };
  const returnPosition = useRef(0);
  const [origin, setOrigin] = useState<PhotoOrigin | undefined>(undefined);
  const returnFocus = useRef<HTMLElement | null>(null);
  const [modeOverride, setMode] = useState<ViewMode | null>(null);
  const mode = modeOverride ?? defaults.galleryView;
  const [viewerInitial, setViewerInitial] = useState<AssetSummary | null>(null);
  const [comparison, setComparison] = useState<AssetSummary[] | null>(null);
  const [organizationIds, setOrganizationIds] = useState<string[] | null>(null);
  const [plansOpen, setPlansOpen] = useState(false);
  function filter(patch: Partial<AssetQuery>) {
    c.filter(patch);
    if (!batchBusy) setViewerInitial(null);
  }
  async function openComparison() {
    const assets = await c.compare();
    if (assets) setComparison(assets);
  }
  const header = (
    <LibraryHeader
      jobsControl={jobsControl}
      navigationControl={navigationControl}
      section={section}
      value={query.name}
      onSearch={(name) => filter({ name })}
      busy={batchBusy}
      onPlans={() => setPlansOpen(true)}
      onFilter={() => setFilterOpen(true)}
      filterOpen={filterOpen}
      activeCount={
        Object.entries(query).filter(
          ([key, value]) => !['sort', 'deferPendingPreviews'].includes(key) && Boolean(value),
        ).length
      }
    />
  );
  return (
    <div className="library-page">
      <div
        className="library-underlay"
        inert={Boolean(viewerInitial)}
        aria-hidden={Boolean(viewerInitial)}
      >
        {header}
        <div className="library-content">
          <PreparationBanner status={status} rootId={query.rootId} onRefresh={onRefresh} />
          <div className="library-content-body">
            {selectionCount > 0 && (
              <BatchToolbar
                loading={loading}
                total={page?.total ?? 0}
                count={selectionCount}
                busy={batchBusy}
                frozen={frozen}
                canCompare={!frozen && selected.size >= 2 && selected.size <= 4}
                onCompare={() => void openComparison()}
                onOrganization={() => setOrganizationIds([...selected])}
                onTags={c.addTags}
                onFreeze={() => void c.freeze()}
                onFavorite={() =>
                  void c.annotate({ favorite: true, rating: null, tags: null }).catch(onError)
                }
                onClear={c.clear}
              />
            )}
            {mode === 'list' && (
              <TextListControls
                columns={columns}
                onColumns={setColumns}
                sort={query.sort}
                onSort={(sort) => filter({ sort })}
                busy={loading || batchBusy}
              />
            )}
            {queryError ? (
              <QueryFailure message={queryError} loading={loading} onRetry={c.retry} />
            ) : (
              <PhotoCollection
                columns={columns}
                display={display}
                viewportHandle={viewportHandle}
                controller={c}
                status={status}
                mode={mode}
                viewerOpen={Boolean(
                  viewerInitial || comparison || filterOpen || organizationIds || plansOpen,
                )}
                onOpen={(id, opener) => {
                  const asset = page?.assets.find((asset) => asset.id === id);
                  if (asset && !loading) {
                    returnPosition.current = opener.closest('.asset-viewport')?.scrollTop ?? 0;
                    returnFocus.current = opener;
                    setOrigin(photoOrigin(opener));
                    setViewerInitial(asset);
                  }
                }}
              />
            )}
            <p className="screen-reader-status" role="status">
              {notice}
            </p>
          </div>
        </div>
      </div>
      {filterOpen && (
        <FilterPanel
          controller={c}
          status={status}
          section={section}
          mode={mode}
          display={display}
          onClose={() => setFilterOpen(false)}
          onSettings={onSettings}
          onRelink={onRelink}
          onApply={(draft, nextMode, nextDisplay) => {
            viewportHandle.current?.remember();
            if (JSON.stringify(draft) !== JSON.stringify(query)) filter(draft);
            setMode(nextMode);
            setDisplay(nextDisplay);
          }}
        />
      )}
      {viewerInitial && (
        <PhotoViewer
          key={viewerInitial.id}
          initial={viewerInitial}
          origin={origin}
          pageAssets={page?.assets ?? []}
          query={query}
          status={status}
          onAnnotate={c.annotate}
          onError={onError}
          onClose={() => {
            setViewerInitial(null);
            restoreAssetFocus(viewerInitial.id, returnFocus.current, returnPosition.current);
          }}
        />
      )}
      {comparison && (
        <ComparisonViewer
          assets={comparison}
          status={status}
          onError={onError}
          onClose={() => setComparison(null)}
        />
      )}
      {organizationIds && (
        <OrganizationPreview
          assetIds={organizationIds}
          revision={status.revision}
          roots={status.roots}
          onClose={() => setOrganizationIds(null)}
        />
      )}
      {plansOpen && <SavedOrganizationPlans onClose={() => setPlansOpen(false)} />}
    </div>
  );
}

function QueryFailure({
  message,
  loading,
  onRetry,
}: {
  message: string;
  loading: boolean;
  onRetry: () => void;
}) {
  return (
    <div className="empty-results">
      <h2>Library couldn’t be loaded</h2>
      <p>{message}</p>
      <button disabled={loading} onClick={onRetry}>
        {loading ? 'Retrying library…' : 'Retry library'}
      </button>
    </div>
  );
}
