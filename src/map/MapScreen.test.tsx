import { useState } from 'react';
import { fireEvent, render, screen, waitFor } from '../test/render';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { AssetQuery, LibraryStatus, MapCluster, MapView, PlaceSummary } from '../bindings';
import { libraryData } from '../ipc';
import { fixtureAsset } from '../library/__tests__/fixtures';
import { MapScreen } from './MapScreen';
import { initialMapState, normalizeViewport, WORLD } from './state';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  library: vi.fn(),
}));
vi.mock('../library/previews', () => ({
  usePreviews: () => ({ results: {}, retry: vi.fn() }),
  previewKey: () => '',
  previewUrl: () => '',
}));
vi.mock('./Basemap', () => ({
  default: ({
    clusters,
    onCluster,
    onViewport,
    onUnavailable,
    updating,
  }: {
    clusters: MapCluster[];
    updating: boolean;
    onCluster: (cluster: MapCluster) => void;
    onViewport: (bounds: typeof WORLD, zoom: number) => void;
    onUnavailable: () => void;
  }) => (
    <div>
      {clusters.map((cluster) => (
        <button key={cluster.key} disabled={updating} onClick={() => onCluster(cluster)}>
          Choose cluster {cluster.count}
        </button>
      ))}
      <button onClick={() => onViewport({ west: 170, east: -170, south: -20, north: 30 }, 5)}>
        Pan across date line
      </button>
      <button onClick={onUnavailable}>Graphics unavailable</button>
    </div>
  ),
}));
vi.mock('../library/PhotoViewer', () => ({
  PhotoViewer: () => <div role="dialog" aria-label="Photo viewer" />,
}));
const asset = fixtureAsset('map-photo');
const cluster: MapCluster = {
  cell: { zoom: 2, x: 2, y: 2 },
  key: 'v1:2:2:2',
  latitude: 0,
  longitude: 0,
  count: 3,
  representative: asset,
};
const view: MapView = {
  clusters: [cluster],
  inView: 3,
  located: 3,
  unavailable: 1,
  clusterZoom: 2,
  revision: '1',
};
const status: LibraryStatus = {
  roots: [
    {
      id: 'root',
      label: 'Generated folder',
      displayPath: 'Generated',
      mode: 'referenced',
      isBase: true,
      availability: 'offline',
      filter: {
        version: 1,
        includeGlobs: [],
        excludeGlobs: [],
        excludedFolders: [],
        extensions: [],
        includeHiddenFiles: false,
        includeHiddenFolders: false,
        minBytes: null,
        maxBytes: null,
        fromDate: null,
        throughDate: null,
      },
      lastScan: null,
      fileCount: 4,
    },
  ],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const show = vi.fn();
const londonPlace: PlaceSummary = {
  id: 'places-v1:city:2643743',
  label: 'London, England, United Kingdom',
  kind: 'city',
  area: 'Within 25 km of the city centre',
  bounds: { west: -0.5, east: 0.3, south: 51.2, north: 51.8 },
};
function Harness({ revision = '1' }: { revision?: string }) {
  const [state, onState] = useState(initialMapState);
  return (
    <MapScreen
      status={{ ...status, revision }}
      state={state}
      onState={onState}
      onShowLibrary={show}
    />
  );
}
beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  show.mockReset();
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'places')
        return {
          matches: request.text ? [londonPlace] : [],
          selected: request.selected_id ? londonPlace : null,
          more: false,
          credits: 'GeoNames · Natural Earth',
        } as never;
      if (request.kind === 'map_view') return view as never;
      return {
        assets: [asset],
        nextCursor: null,
        total: request.kind === 'query' && request.query.location === 'unavailable' ? 1 : 3,
        revision: '1',
      } as never;
    });
});
describe('local map browsing', () => {
  it('loads photo results only after a marker click and keeps markers during viewport requests', async () => {
    render(<Harness />);
    const marker = await screen.findByRole('button', { name: 'Choose cluster 3' });
    expect(
      screen.getByText('Click a photo marker to show photos from that location.'),
    ).toBeVisible();
    expect(vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'query')).toBe(
      false,
    );
    fireEvent.click(marker);
    await screen.findByRole('button', { name: `Open ${asset.name}` });
    const mapCalls = vi.mocked(libraryData).mock.calls.filter(([r]) => r.kind === 'map_view');
    expect(mapCalls).toHaveLength(1);
    let resolveMap: ((value: MapView) => void) | undefined;
    vi.mocked(libraryData).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveMap = resolve as (value: MapView) => void;
        }),
    );
    const queryCount = vi.mocked(libraryData).mock.calls.filter(([r]) => r.kind === 'query').length;
    fireEvent.click(screen.getByRole('button', { name: 'Pan across date line' }));
    expect(marker).toBeInTheDocument();
    expect(marker).toBeDisabled();
    expect(screen.queryByRole('button', { name: `Open ${asset.name}` })).not.toBeInTheDocument();
    expect(screen.queryByText('Loading photos…')).not.toBeInTheDocument();
    await waitFor(() => expect(resolveMap).toBeDefined());
    resolveMap?.(view);
    await waitFor(() => expect(marker).toBeEnabled());
    expect(vi.mocked(libraryData).mock.calls.filter(([r]) => r.kind === 'query')).toHaveLength(
      queryCount,
    );
    fireEvent.click(marker);
    await screen.findByRole('button', { name: `Open ${asset.name}` });
  });
  it('lets an explicit selection expand to all photos in view', async () => {
    render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Choose cluster 3' }));
    await screen.findByRole('button', { name: `Open ${asset.name}` });
    fireEvent.click(screen.getByRole('button', { name: 'All photos in view' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'query',
          query: expect.objectContaining({ mapCell: null, mapBounds: WORLD }),
        }),
        'page',
      ),
    );
    expect(await screen.findByRole('button', { name: `Open ${asset.name}` })).toBeVisible();
  });
  it('chooses an offline place from the main search and carries its identity into Photos', async () => {
    render(<Harness />);
    fireEvent.change(screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }), {
      target: { value: '10-10-2025' },
    });
    fireEvent.focus(screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }));
    fireEvent.change(
      await screen.findByRole('searchbox', { name: 'Search filenames, dates or places' }),
      {
        target: { value: 'London' },
      },
    );
    fireEvent.click(await screen.findByRole('button', { name: /London, England, United Kingdom/ }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'query',
          query: expect.objectContaining({
            name: '',
            placeId: londonPlace.id,
            mapBounds: londonPlace.bounds,
            location: 'known',
          }),
        }),
        'page',
      ),
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Show in library' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Show in library' }));
    expect(show).toHaveBeenCalledWith(
      expect.objectContaining({ name: '', placeId: londonPlace.id }),
      asset.id,
    );
    fireEvent.focus(screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Clear place filter' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'query',
          query: expect.objectContaining({ name: '', placeId: null }),
        }),
        'page',
      ),
    );
  });
  it('shares date search text between map membership and photo queries and can clear it', async () => {
    render(<Harness />);
    const search = screen.getByRole('searchbox', { name: 'Search filenames, dates or places' });
    expect(search).toHaveAttribute('title', expect.stringContaining('DD-MM-YYYY'));
    fireEvent.change(search, { target: { value: '10-10-2025' } });
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    for (const [kind, result] of [
      ['map_view', 'map_view'],
      ['query', 'page'],
    ] as const) {
      await waitFor(() =>
        expect(libraryData).toHaveBeenCalledWith(
          expect.objectContaining({
            kind,
            query: expect.objectContaining({ name: '10-10-2025', includeMissing: false }),
          }),
          result,
        ),
      );
    }
    fireEvent.change(search, { target: { value: '' } });
    await waitFor(() => expect(search).toHaveValue(''));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenLastCalledWith(
        expect.objectContaining({ kind: 'query', query: expect.objectContaining({ name: '' }) }),
        'page',
      ),
    );
  });
  it('keeps an open viewer through catalog refreshes without losing its filter scope', async () => {
    const { rerender } = render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Choose cluster 3' }));
    fireEvent.click(await screen.findByRole('button', { name: `Open ${asset.name}` }));
    expect(screen.getByRole('dialog', { name: 'Photo viewer' })).toBeVisible();
    rerender(<Harness revision="2" />);
    expect(screen.getByRole('dialog', { name: 'Photo viewer' })).toBeVisible();
  });
  it('carries the exact cluster, viewport and selection to the library', async () => {
    render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Choose cluster 3' }));
    await screen.findByRole('heading', { name: 'Selected location' });
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Show in library' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Show in library' }));
    expect(show).toHaveBeenCalledWith(
      expect.objectContaining({ mapBounds: WORLD, mapCell: cluster.cell, location: 'known' }),
      'map-photo',
    );
  });
  it('uses native folder and date filters without treating offline originals as missing', async () => {
    render(<Harness />);
    await screen.findByRole('button', { name: 'Choose cluster 3' });
    fireEvent.click(screen.getByRole('button', { name: 'Map filters' }));
    await screen.findByRole('dialog', { name: 'Map filters' });
    fireEvent.change(await screen.findByLabelText('Library folder'), { target: { value: 'root' } });
    fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-01-01' } });
    fireEvent.change(screen.getByLabelText('Through'), { target: { value: '2026-11-01' } });
    fireEvent.click(screen.getByRole('button', { name: 'Close filters' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'query',
          query: expect.objectContaining({
            rootId: 'root',
            fromDate: '2026-01-01',
            throughDate: '2026-11-01',
            includeMissing: false,
          }),
        }),
        'page',
      ),
    );
    expect(screen.getByText(/Offline originals remain offline/)).toBeVisible();
  });
  it('lists absent or invalid locations with no zero-coordinate substitution', async () => {
    render(<Harness />);
    await screen.findByRole('button', { name: 'Choose cluster 3' });
    fireEvent.click(screen.getByRole('button', { name: 'Map filters' }));
    fireEvent.click(await screen.findByRole('button', { name: 'No usable location (1)' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'query',
          query: expect.objectContaining({
            location: 'unavailable',
            mapBounds: null,
            mapCell: null,
          }),
        }),
        'page',
      ),
    );
    expect(screen.queryByRole('button', { name: 'Choose cluster 3' })).not.toBeInTheDocument();
    expect(screen.queryByRole('region', { name: 'Geographic overview' })).not.toBeInTheDocument();
    expect(await screen.findByText('1 photo without usable location')).toBeVisible();
  });
  it('preserves crossing viewports and keeps bounded paging', async () => {
    render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Pan across date line' }));
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'map_view',
          bounds: { west: 170, east: -170, south: -20, north: 30 },
          zoom: 5,
        }),
        'map_view',
      ),
    );
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Choose cluster 3' })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole('button', { name: 'Choose cluster 3' }));
    await screen.findByRole('button', { name: `Open ${asset.name}` });
    const queries = vi.mocked(libraryData).mock.calls.filter(([r]) => r.kind === 'query');
    expect(queries.every(([r]) => r.kind === 'query' && r.limit === 80)).toBe(true);
  });
  it('keeps the photo list usable when graphics fail', async () => {
    render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Graphics unavailable' }));
    expect(await screen.findByText(/map could not be displayed/)).toBeVisible();
    expect(await screen.findByRole('button', { name: `Open ${asset.name}` })).toBeEnabled();
  });
  it('reports native failures and retries without fake empty success', async () => {
    vi.mocked(libraryData).mockRejectedValueOnce(new Error('Catalog unavailable.'));
    render(<Harness />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable.');
    expect(screen.getByRole('button', { name: 'Show in library' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Retry photos' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Choose cluster 3' }));
    await waitFor(() =>
      expect(screen.getByRole('button', { name: 'Show in library' })).toBeEnabled(),
    );
  });
  it('keeps markers when a selected photo request fails and rejects late photo results after panning', async () => {
    render(<Harness />);
    const marker = await screen.findByRole('button', { name: 'Choose cluster 3' });
    vi.mocked(libraryData).mockRejectedValueOnce(new Error('Photo page unavailable.'));
    fireEvent.click(marker);
    expect(await screen.findByRole('alert')).toHaveTextContent('Photo page unavailable.');
    expect(marker).toBeEnabled();
    expect(vi.mocked(libraryData).mock.calls.filter(([r]) => r.kind === 'map_view')).toHaveLength(
      1,
    );
    let resolvePage: ((value: unknown) => void) | undefined;
    vi.mocked(libraryData)
      .mockImplementationOnce(async () => view as never)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolvePage = resolve as (value: unknown) => void;
          }),
      );
    fireEvent.click(screen.getByRole('button', { name: 'Retry photos' }));
    await waitFor(() => expect(resolvePage).toBeDefined());
    fireEvent.click(screen.getByRole('button', { name: 'Pan across date line' }));
    resolvePage?.({ assets: [asset], nextCursor: null, total: 3, revision: '1' });
    await waitFor(() => expect(marker).toBeEnabled());
    expect(screen.queryByRole('button', { name: `Open ${asset.name}` })).not.toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });
  it('refuses mismatched catalog generations', async () => {
    vi.mocked(libraryData).mockImplementation(async (request) =>
      request.kind === 'map_view'
        ? (view as never)
        : ({ assets: [asset], nextCursor: null, total: 1, revision: '2' } as never),
    );
    render(<Harness />);
    fireEvent.click(await screen.findByRole('button', { name: 'Choose cluster 3' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('catalog changed');
    expect(screen.queryByRole('button', { name: `Open ${asset.name}` })).not.toBeInTheDocument();
  });
  it('discards a late response after the filter scope changes', async () => {
    let resolveFirst: ((value: MapView) => void) | undefined;
    vi.mocked(libraryData).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveFirst = resolve as (value: MapView) => void;
        }),
    );
    render(<Harness />);
    await waitFor(() => expect(resolveFirst).toBeDefined());
    fireEvent.change(screen.getByRole('searchbox'), { target: { value: 'new-scope' } });
    await waitFor(() =>
      expect(libraryData).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'map_view',
          query: expect.objectContaining({ name: 'new-scope' }),
        }),
        'map_view',
      ),
    );
    resolveFirst?.({ ...view, clusters: [{ ...cluster, count: 99 }] });
    expect(await screen.findByRole('button', { name: 'Choose cluster 3' })).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Choose cluster 99' })).not.toBeInTheDocument();
  });
});
describe('viewport normalization', () => {
  it('normalizes wrapped worlds and retains polar photo lists', () => {
    expect(normalizeViewport(170, -20, 190, 30)).toEqual({
      west: 170,
      east: -170,
      south: -20,
      north: 30,
    });
    expect(normalizeViewport(-190, -86, 190, 86)).toEqual(WORLD);
    const query: AssetQuery = { ...initialMapState().query, mapBounds: WORLD };
    expect(query.includeMissing).toBe(false);
  });
});
