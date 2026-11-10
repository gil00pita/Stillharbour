import { Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { MapFilters } from '@/components/MapFilters';
import { lazy, Suspense, useCallback, useEffect, useMemo, useState, type ReactNode } from 'react';
import type {
  AssetPage,
  AssetQuery,
  AssetSummary,
  LibraryStatus,
  MapBounds,
  MapCluster,
  MapView,
} from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { PhotoViewer } from '@/components/PhotoViewer';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { previewKey, previewUrl, usePreviews } from '../../library/previews';
import { mapPhotoQuery, WORLD, type MapBrowseState } from '../../map/state';
import { PlaceSearch } from '@/components/PlaceSearch';

const Basemap = lazy(() => import('@/components/Basemap'));
export function MapScreen({
  status,
  state,
  onState,
  onShowLibrary,
  navigationControl,
  jobsControl,
}: {
  status: LibraryStatus;
  state: MapBrowseState;
  onState: (value: MapBrowseState) => void;
  onShowLibrary: (query: AssetQuery, selectedId: string | null) => void;
  navigationControl?: ReactNode;
  jobsControl?: ReactNode;
}) {
  const photoQuery = useMemo(() => mapPhotoQuery(state), [state]);
  const viewerScope = JSON.stringify(photoQuery);
  const mapScope = JSON.stringify([state.query, status.revision]);
  const mapKey = JSON.stringify([mapScope, state.bounds, state.zoom]);
  const [mapResponse, setMapResponse] = useState<{
    key: string;
    scope: string;
    view: MapView;
  } | null>(null);
  const currentMap = mapResponse?.key === mapKey ? mapResponse.view : null;
  // Retain markers through camera updates, but never across filter/catalog changes.
  const visibleMap = mapResponse?.scope === mapScope ? mapResponse.view : null;
  const [allPhotosScope, setAllPhotosScope] = useState<string | null>(null);

  const scope = JSON.stringify([photoQuery, state.zoom, status.revision]);
  const [cursor, setCursor] = useState<{ scope: string; value: string } | null>(null);
  const pageCursor = cursor?.scope === scope ? cursor.value : null;
  const key = JSON.stringify([scope, pageCursor]);
  const [attempt, setAttempt] = useState(0);
  const [response, setResponse] = useState<{ key: string; page: AssetPage } | null>(null);
  const [failure, setFailure] = useState<{ key: string; message: string } | null>(null);
  const [mapUnavailable, setMapUnavailable] = useState(false);
  const photosRequested = Boolean(
    state.cell || state.unlocated || mapUnavailable || allPhotosScope === viewerScope,
  );
  const [mapFailure, setMapFailure] = useState<{ key: string; message: string } | null>(null);
  const [placeView, setPlaceView] = useState<{ bounds: MapBounds; zoom: number } | null>(null);
  const [viewer, setViewer] = useState<{ scope: string; asset: AssetSummary } | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    const timer = setTimeout(() => {
      void libraryData(
        { kind: 'map_view', query: state.query, bounds: state.bounds, zoom: state.zoom },
        'map_view',
      )
        .then((view) => {
          if (active) {
            setMapResponse({ key: mapKey, scope: mapScope, view });
            setMapFailure(null);
          }
        })
        .catch((error: unknown) => {
          if (active) setMapFailure({ key: mapKey, message: errorMessage(error) });
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [mapKey, mapScope, attempt, state.query, state.bounds, state.zoom]);
  useEffect(() => {
    if (!photosRequested || !currentMap) return;
    let active = true;
    const timer = setTimeout(() => {
      void libraryData({ kind: 'query', query: photoQuery, cursor: pageCursor, limit: 80 }, 'page')
        .then((page) => {
          if (currentMap.revision !== page.revision)
            throw new Error('The catalog changed while the map loaded. Retry photos.');
          if (active) {
            setResponse({ key, page });
            setFailure(null);
          }
        })
        .catch((error: unknown) => {
          if (active) setFailure({ key, message: errorMessage(error) });
        });
    }, 150);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [key, attempt, photosRequested, currentMap, photoQuery, pageCursor]);
  const current =
    photosRequested && response?.key === key && response.page.revision === currentMap?.revision
      ? response
      : null;
  const error =
    (mapFailure?.key === mapKey ? mapFailure.message : null) ??
    (photosRequested && failure?.key === key ? failure.message : null);
  const previewFailure = useCallback((error: unknown) => setPreviewError(errorMessage(error)), []);
  const assets = current?.page.assets ?? [];
  const clusters = visibleMap?.clusters ?? [];
  const keys = [...clusters.map((c) => c.representative), ...assets].map((asset) => ({
    assetId: asset.id,
    revision: asset.revision,
    edge: 256,
  }));
  const previews = usePreviews(keys, previewFailure, Boolean(visibleMap));
  const unavailable = useCallback(() => setMapUnavailable(true), []);
  function filter(patch: Partial<AssetQuery>) {
    setAllPhotosScope(null);
    onState({ ...state, query: { ...state.query, ...patch }, cell: null, selectedId: null });
  }
  function viewport(bounds: MapBounds, zoom: number) {
    if (
      zoom === state.zoom &&
      (['west', 'east', 'south', 'north'] as const).every(
        (key) => Math.abs(bounds[key] - state.bounds[key]) < 1e-7,
      )
    )
      return;
    setAllPhotosScope(null);
    onState({ ...state, bounds, zoom, cell: null, selectedId: null, unlocated: false });
  }
  function cluster(value: MapCluster) {
    onState({ ...state, cell: value.cell, selectedId: value.representative.id, unlocated: false });
  }
  const activeViewer = viewer?.scope === viewerScope ? viewer.asset : null;
  return (
    <Box
      className="library-page map-page"
      css={{
        display: 'flex',
        flexDirection: 'column',
        flex: '1',
        minWidth: '0',
        minHeight: '0',
        position: 'relative',
        overflow: 'hidden',
        isolation: 'isolate',
        '& .map-world': {
          position: 'absolute',
          zIndex: '2',
          top: '12px',
          left: '12px',
          width: '36px',
          height: '36px',
          padding: '0',
        },
        '& .map-zoom button:not(.primary):not(.asset-open)': {
          width: '36px',
          height: '36px',
          padding: '0',
          fontSize: '22px',
          lineHeight: '1',
        },
        '& button.map-marker:not(.primary):not(.asset-open)': {
          display: 'grid',
          placeItems: 'center',
          gridTemplate: 'minmax(0, 1fr) / minmax(0, 1fr)',
          position: 'relative',
          padding: '4px',
          width: '48px',
          height: '48px',
          border: '2px solid {colors.fg}',
          borderRadius: '8px',
          background: '{colors.border.control}',
          color: 'white',
          boxShadow: '0 3px 12px #0008',
        },
        '& button.map-marker:disabled': { opacity: '1' },
        '& .map-filmstrip button:not(.primary):not(.asset-open)': {
          width: '100%',
          display: 'grid',
          gap: '8px',
          textAlign: 'left',
          padding: '0',
          border: '0',
          background: 'transparent',
        },
        '& .photo-toolbar h1': { display: 'block' },
        '& .map-filmstrip button:hover:enabled': { background: '{colors.bg.hover}' },
      }}
    >
      <Box
        className="map-underlay"
        css={{
          display: 'flex',
          flexDirection: 'column',
          minHeight: '0',
          flex: '1',
          overflow: 'auto',
        }}
        inert={Boolean(activeViewer)}
      >
        <PageHeader>
          <chakra.header
            className="toolbar photo-toolbar"
            css={{
              flexShrink: '0',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '16px',
              minHeight: '64px',
              padding: '12px 24px',
              borderBottom: '1px solid {colors.border}',
              background: '{colors.bg.toolbar}',
              '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
              '& h1': { flex: '0 0 auto' },
              '& > .toolbar-secondary': {
                background: 'transparent',
                borderColor: 'transparent',
                color: '{colors.fg.muted}',
                padding: '7px 12px',
              },
              '& .jobs-trigger': {
                background: 'transparent',
                borderColor: 'transparent',
                color: '{colors.fg.muted}',
              },
              '& > .toolbar-secondary:hover:enabled': {
                background: '{colors.bg.hover}',
                color: 'var(--stillharbour-colors-fg, {colors.fg})',
              },
              '& .jobs-trigger:hover:enabled': {
                background: '{colors.bg.hover}',
                color: 'var(--stillharbour-colors-fg, {colors.fg})',
              },
              '@media (max-width: 760px)': {
                '& .toolbar-action-label': { display: 'none' },
                '& > .toolbar-secondary': { width: 'var(--control-height)', padding: '0' },
              },
            }}
          >
            {navigationControl}
            <Heading as="h1" unstyled>
              Map
            </Heading>
            <PlaceSearch
              value={state.query.name}
              onSearch={(name) => filter({ name })}
              selectedId={state.query.placeId}
              onChange={(place) => {
                setAllPhotosScope(null);
                setPlaceView(
                  place ? { bounds: place.bounds, zoom: place.kind === 'city' ? 10 : 3 } : null,
                );
                onState({
                  ...state,
                  query: {
                    ...state.query,
                    name: place ? '' : state.query.name,
                    placeId: place?.id ?? null,
                  },
                  bounds: place?.bounds ?? state.bounds,
                  zoom: place ? (place.kind === 'city' ? 10 : 3) : state.zoom,
                  cell: null,
                  selectedId: null,
                  unlocated: false,
                });
              }}
            />
            <Box
              className="header-actions"
              css={{ display: 'flex', alignItems: 'center', gap: '8px' }}
            >
              <MapFilters
                query={state.query}
                roots={status.roots}
                unlocated={state.unlocated}
                unavailable={visibleMap?.unavailable ?? null}
                onFilter={filter}
                onLocation={() => {
                  setAllPhotosScope(null);
                  onState({ ...state, unlocated: !state.unlocated, cell: null, selectedId: null });
                }}
              />
              {jobsControl}
            </Box>
          </chakra.header>
        </PageHeader>
        {!state.unlocated && (
          <chakra.section
            className="map-geography"
            css={{
              position: 'relative',
              height: 'clamp(260px, 45vh, 520px)',
              flexShrink: '0',
              background: '{colors.bg.sidebar}',
              '@media (max-width: 760px)': { height: '280px' },
            }}
            aria-label="Geographic overview"
          >
            {mapUnavailable ? (
              <Box
                className="map-fallback"
                css={{ padding: '28px', color: '{colors.fg}' }}
                role="status"
              >
                <Text>
                  The map could not be displayed. Your photos and location filters remain available
                  below.
                </Text>
                <Button onClick={() => onState({ ...state, bounds: WORLD, zoom: 0, cell: null })}>
                  Show all locations
                </Button>
              </Box>
            ) : (
              <Suspense fallback={<Text role="status">Loading offline map…</Text>}>
                <Basemap
                  bounds={state.bounds}
                  zoom={state.zoom}
                  clusters={state.unlocated ? [] : clusters}
                  previews={previews.results}
                  selectedId={state.selectedId}
                  updating={!currentMap}
                  onViewport={viewport}
                  onCluster={cluster}
                  onUnavailable={unavailable}
                  placeView={placeView}
                />
              </Suspense>
            )}
          </chakra.section>
        )}
        <chakra.section
          className="map-photos"
          css={{ padding: '20px 24px', '@media (max-width: 760px)': { padding: '12px' } }}
          aria-label={state.unlocated ? 'Photos without usable location' : 'Photos in map area'}
          aria-busy={photosRequested && !current && !error}
        >
          <Box
            className="map-photo-heading"
            css={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: '16px',
              alignItems: 'center',
              marginBottom: '16px',
              '& > div:first-child': { marginRight: 'auto' },
              '& h2': { margin: '0 0 8px', fontSize: '16px', fontWeight: '550' },
              '& p': { margin: '0', color: '{colors.fg.muted}', fontSize: '13px' },
            }}
          >
            <Box>
              <Heading as="h2" unstyled>
                {state.unlocated
                  ? 'No usable location'
                  : state.cell
                    ? 'Selected location'
                    : 'Photos in view'}
              </Heading>
              <Text role="status">
                {error
                  ? 'Photos could not be loaded.'
                  : current
                    ? state.unlocated
                      ? `${current.page.total.toLocaleString()} ${current.page.total === 1 ? 'photo' : 'photos'} without usable location`
                      : `${current.page.total.toLocaleString()} photos in view · ${(currentMap?.located ?? 0).toLocaleString()} located in these folders`
                    : photosRequested
                      ? 'Loading photos…'
                      : 'Click a photo marker to show photos from that location.'}
              </Text>
            </Box>
            <Button
              disabled={!current || current.page.total === 0}
              onClick={() => onShowLibrary(photoQuery, state.selectedId)}
            >
              Show in photos
            </Button>
            {!photosRequested && (
              <Button onClick={() => setAllPhotosScope(viewerScope)}>Show photos in view</Button>
            )}
            {state.cell && (
              <Button
                onClick={() => {
                  const next = { ...state, cell: null, selectedId: null };
                  setAllPhotosScope(JSON.stringify(mapPhotoQuery(next)));
                  onState(next);
                }}
              >
                All photos in view
              </Button>
            )}
          </Box>
          {error && (
            <Box role="alert">
              <Text>{error}</Text>
              <Button onClick={() => setAttempt((v) => v + 1)}>Retry photos</Button>
            </Box>
          )}
          {previewError && (
            <Text role="alert">
              Some previews could not be loaded.{' '}
              <Button
                onClick={() => {
                  setPreviewError(null);
                  previews.retry();
                }}
              >
                Retry previews
              </Button>
            </Text>
          )}
          {current && !assets.length && (
            <Text>
              {state.unlocated
                ? 'Every photo in these filters has a usable catalog location.'
                : 'No located photos in this area. Try another area or view photos with no usable location.'}
            </Text>
          )}
          <List.Root
            unstyled
            className="map-filmstrip"
            css={{
              display: 'flex',
              overflow: 'auto',
              gap: '12px',
              listStyle: 'none',
              margin: '0',
              padding: '4px 4px 16px',
              '& li': { flex: '0 0 132px', minWidth: '0' },
              '& img': {
                display: 'grid',
                placeItems: 'center',
                width: '132px',
                height: '92px',
                objectFit: 'cover',
                borderRadius: '4px',
                background: '{colors.bg.panel}',
              },
              '& button > span:last-child': {
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                width: '132px',
                fontSize: '13px',
              },
            }}
          >
            {assets.map((asset) => {
              const preview =
                previews.results[
                  previewKey({ assetId: asset.id, revision: asset.revision, edge: 256 })
                ];
              const edge = preview?.state === 'ready' ? 256 : preview?.fallbackEdge;
              return (
                <List.Item unstyled key={asset.id}>
                  <Button
                    aria-label={`Open ${asset.name}`}
                    onClick={() => setViewer({ scope: viewerScope, asset })}
                  >
                    {edge ? (
                      <CachedPreviewImage src={previewUrl(asset, edge)} alt="" />
                    ) : (
                      <Text
                        as="span"
                        className="map-photo-placeholder"
                        css={{
                          display: 'grid',
                          placeItems: 'center',
                          width: '132px',
                          height: '92px',
                          objectFit: 'cover',
                          borderRadius: '4px',
                          background: '{colors.bg.panel}',
                        }}
                        aria-hidden="true"
                      >
                        ◇
                      </Text>
                    )}
                    <Text as="span">{asset.name}</Text>
                  </Button>
                </List.Item>
              );
            })}
          </List.Root>
          {current && (
            <Box className="map-paging" css={{ display: 'flex', gap: '12px', padding: '8px 0' }}>
              {pageCursor && <Button onClick={() => setCursor(null)}>First photos</Button>}
              {current.page.nextCursor && (
                <Button onClick={() => setCursor({ scope, value: current.page.nextCursor! })}>
                  More photos
                </Button>
              )}
            </Box>
          )}
          <Text
            className="map-disclosure"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '1.6',
              maxWidth: '100ch',
            }}
          >
            Locations are last observed catalog metadata. Offline originals remain offline. Missing,
            invalid or unextracted coordinates appear under “No usable location”. The offline
            overview has no street detail; polar markers are shown at the map edge. Satellite is an
            online NASA overview with limited detail. Selecting it sends viewed map areas to NASA;
            your photos are not sent.
          </Text>
        </chakra.section>
      </Box>
      {activeViewer && (
        <PhotoViewer
          key={`${activeViewer.id}:${viewerScope}`}
          initial={activeViewer}
          pageAssets={assets}
          query={photoQuery}
          status={status}
          onClose={() => setViewer(null)}
          onError={previewFailure}
          onAnnotate={async (patch, id) => {
            await library({
              kind: 'annotate',
              selection: { kind: 'ids', ids: [id ?? activeViewer.id] },
              patch,
            });
            setAttempt((v) => v + 1);
          }}
        />
      )}
    </Box>
  );
}
