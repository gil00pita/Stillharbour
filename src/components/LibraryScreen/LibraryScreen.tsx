import { mapFilterLabel } from '@/utils/map-filter-label';
import { PhotoAlbumActions } from '@/components/PhotoAlbumActions';
import { Button } from '@/components/Button';
import { useCallback, useRef, useState, type ReactNode } from 'react';
import type { AssetSummary, AssetQuery, LibraryStatus } from '../../bindings';
import { FilterPanel, type GalleryDisplay } from '@/components/FilterPanel';
import type { PhotoViewportHandle } from '@/components/AssetBrowser';
import { PhotoCollection } from '@/components/PhotoCollection';
import { useLibraryController } from '@/hooks/use-library-controller';
import { LibraryHeader } from '@/components/LibraryHeader';
import { PhotoViewer } from '@/components/PhotoViewer';
import { ComparisonViewer } from '@/components/ComparisonViewer';
import { OrganizationPreview } from '@/components/OrganizationPreview';
import { BatchToolbar } from '@/components/BatchToolbar';
import { TextListControls } from '@/components/TextList';
import { PreparationBanner } from '@/components/PreparationBanner';
import { DEFAULT_TEXT_COLUMNS } from '../../library/textColumns';
import { photoOrigin, type PhotoOrigin } from '../../library/viewerMotion';
import { restoreAssetFocus } from '../../library/focus';
import { CatalogOrganizer } from '@/components/CatalogOrganizer';
import { type ViewMode, type Section } from '../../library/model';

export type { Section } from '../../library/model';
export function LibraryScreen({
  albumControls,
  initialQuery,
  initialSelectedId,
  onReturnToMap,
  catalogId = null,
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
  albumControls?: ReactNode;
  initialQuery?: AssetQuery | undefined;
  initialSelectedId?: string | null | undefined;
  onReturnToMap?: (() => void) | undefined;
  catalogId?: string | null;
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  defaults: import('../../bindings').AppPreferences;
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
    initialQuery,
    initialSelectedId: initialSelectedId ?? null,
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
  const [selectionMode, setSelectionMode] = useState(false);
  const selecting = selectionMode || selectionCount > 0 || frozen;
  const [catalogOpen, setCatalogOpen] = useState(false);
  function filter(patch: Partial<AssetQuery>) {
    c.filter(patch);
    if (!batchBusy) setViewerInitial(null);
  }
  async function openComparison() {
    const assets = await c.compare();
    if (assets) setComparison(assets);
  }
  const mapScope = mapFilterLabel(query);
  const clearMapScope = () => filter({ mapBounds: null, mapCell: null, location: null });
  const header = (
    <LibraryHeader
      jobsControl={jobsControl}
      navigationControl={navigationControl}
      section={section}
      value={query.name}
      onSearch={(name) => filter({ name })}
      placeId={query.placeId}
      mapScope={mapScope ? { label: mapScope, onClear: clearMapScope } : undefined}
      onPlace={(place) => filter({ placeId: place?.id ?? null, ...(place ? { name: '' } : {}) })}
      busy={batchBusy}
      selecting={selecting}
      onSelect={() => {
        if (selecting) {
          c.clear();
          setSelectionMode(false);
        } else setSelectionMode(true);
      }}
      onFilter={() => setFilterOpen(true)}
      filterOpen={filterOpen}
      activeCount={
        Object.entries(query).filter(
          ([key, value]) =>
            !['sort', 'deferPendingPreviews', 'mapBounds', 'mapCell', 'location'].includes(key) &&
            (Boolean(value) || (key === 'rejected' && value !== null && value !== undefined)),
        ).length + (mapScope ? 1 : 0)
      }
    />
  );
  return (
    <PhotoAlbumActions
      status={status}
      onChanged={onRefresh}
      selection={{
        active: selecting,
        disabled: frozen || batchBusy,
        onSelect: (asset) => {
          setSelectionMode(true);
          if (!selected.has(asset.id)) c.select(asset.id, false);
        },
      }}
    >
      <div className={`library-page ${selecting ? 'photo-selection-mode' : ''}`}>
        <div
          className="library-underlay"
          inert={Boolean(viewerInitial)}
          aria-hidden={Boolean(viewerInitial)}
        >
          {header}
          {albumControls}
          {!albumControls && query.albumId && (
            <div className="catalog-collection-scope">
              <span>Virtual album filter active</span>
              <Button onClick={() => filter({ albumId: null })}>Clear album filter</Button>
            </div>
          )}
          {mapScope && (
            <div className="map-filters">
              <span>
                {query.location === 'unavailable'
                  ? 'Photos with no usable location'
                  : 'Photos from the selected map area'}
              </span>
              {onReturnToMap && <Button onClick={onReturnToMap}>Return to map</Button>}
              <Button onClick={clearMapScope}>Clear location filter</Button>
            </div>
          )}
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
                  onAlbum={() => setCatalogOpen(true)}
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
                <QueryFailure loading={loading} onRetry={c.retry} />
              ) : (
                <PhotoCollection
                  columns={columns}
                  display={display}
                  viewportHandle={viewportHandle}
                  controller={c}
                  status={status}
                  mode={mode}
                  viewerOpen={Boolean(
                    viewerInitial || comparison || filterOpen || organizationIds || catalogOpen,
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
        {catalogOpen && (
          <CatalogOrganizer
            roots={status.roots}
            query={query}
            selection={c.selection.scope}
            selectionCount={selectionCount}
            onBrowse={filter}
            onChanged={onRefresh}
            onClose={() => setCatalogOpen(false)}
          />
        )}
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
            catalogId={catalogId}
            preferredImportRoot={status.importDestination ?? null}
            assetIds={organizationIds}
            revision={status.revision}
            roots={status.roots}
            onClose={() => setOrganizationIds(null)}
          />
        )}
      </div>
    </PhotoAlbumActions>
  );
}

function QueryFailure({ loading, onRetry }: { loading: boolean; onRetry: () => void }) {
  return (
    <div className="empty-results">
      <h2>Library couldn’t be loaded</h2>
      <Button disabled={loading} onClick={onRetry}>
        {loading ? 'Retrying library…' : 'Retry library'}
      </Button>
    </div>
  );
}
