import type { MapOptions } from 'maplibre-gl';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '../test/render';
import Basemap from './Basemap';
import { WORLD } from './state';
import { SATELLITE_SOURCE, SATELLITE_TILES, satelliteRequest } from './satellite';

const mock = vi.hoisted(() => ({
  maps: [] as { options: MapOptions; events: Record<string, (event?: unknown) => void> }[],
  sources: new Set<string>(),
  layers: new Set<string>(),
  addSource: vi.fn(),
  addLayer: vi.fn(),
  removeSource: vi.fn(),
  removeLayer: vi.fn(),
  jumpTo: vi.fn(),
}));
vi.mock('maplibre-gl', () => ({
  setWorkerCount: vi.fn(),
  setWorkerUrl: vi.fn(),
  setMaxParallelImageRequests: vi.fn(),
  Map: class {
    events: Record<string, (event?: unknown) => void> = {};
    constructor(public options: MapOptions) {
      mock.maps.push(this);
    }
    on(event: string, callback: (event?: unknown) => void) {
      this.events[event] = callback;
    }
    getCanvas() {
      return document.createElement('canvas');
    }
    resize() {}
    remove() {}
    jumpTo = mock.jumpTo;
    addSource(id: string, source: unknown) {
      mock.addSource(id, source);
      mock.sources.add(id);
    }
    addLayer(layer: { id: string }) {
      mock.addLayer(layer);
      mock.layers.add(layer.id);
    }
    getSource(id: string) {
      return mock.sources.has(id);
    }
    getLayer(id: string) {
      return mock.layers.has(id);
    }
    removeSource(id: string) {
      mock.removeSource(id);
      mock.sources.delete(id);
    }
    removeLayer(id: string) {
      mock.removeLayer(id);
      mock.layers.delete(id);
    }
  },
}));
vi.mock('maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url', () => ({ default: '/worker.js' }));

beforeEach(() => {
  vi.clearAllMocks();
  mock.maps.length = 0;
  mock.sources.clear();
  mock.layers.clear();
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
});
function mount() {
  const onViewport = vi.fn();
  const onUnavailable = vi.fn();
  render(
    <Basemap
      bounds={WORLD}
      zoom={0}
      clusters={[]}
      previews={{}}
      selectedId={null}
      updating={false}
      onViewport={onViewport}
      onCluster={vi.fn()}
      onUnavailable={onUnavailable}
      placeView={null}
    />,
  );
  const instance = mock.maps[0]!;
  act(() => instance.events.load?.());
  return { onViewport, onUnavailable, instance };
}
describe('map view controls', () => {
  it('uses a named globe icon to reset the current map without visible label text', () => {
    const { onViewport } = mount();
    const globe = screen.getByRole('button', { name: 'Whole world' });
    expect(globe).toHaveTextContent('');
    expect(globe.querySelector('svg')).not.toBeNull();
    fireEvent.click(globe);
    expect(mock.jumpTo).toHaveBeenCalledWith({ center: [0, 20], zoom: 0 });
    expect(onViewport).toHaveBeenCalledWith(WORLD, 0);
  });
  it('keeps NASA unavailable until selection and switches layers without moving or recreating the map', () => {
    const { instance, onViewport } = mount();
    const transform = instance.options.transformRequest!;
    const url = SATELLITE_TILES.tiles![0]!.replace('{z}', '0')
      .replace('{y}', '0')
      .replace('{x}', '0');
    expect(() => transform(url)).toThrow();
    expect(mock.addSource).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Satellite' }));
    expect(mock.addSource).toHaveBeenCalledWith(SATELLITE_SOURCE, SATELLITE_TILES);
    expect(screen.getByRole('button', { name: 'Satellite' })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByText(/limited detail · internet required/)).toBeVisible();
    expect(transform(url)).toEqual(
      expect.objectContaining({
        cache: 'no-store',
        credentials: 'same-origin',
        referrerPolicy: 'no-referrer',
      }),
    );
    act(() =>
      instance.events.sourcedata?.({
        sourceId: SATELLITE_SOURCE,
        isSourceLoaded: true,
        sourceDataType: 'metadata',
      }),
    );
    expect(screen.getByRole('status')).toHaveTextContent('Loading NASA');
    act(() =>
      instance.events.sourcedata?.({
        sourceId: SATELLITE_SOURCE,
        isSourceLoaded: true,
        tile: {},
      }),
    );
    expect(screen.getByRole('status')).not.toHaveTextContent('Loading');
    act(() => instance.events.sourcedataloading?.({ sourceId: SATELLITE_SOURCE }));
    expect(screen.getByRole('status')).toHaveTextContent('Loading NASA');
    act(() =>
      instance.events.sourcedata?.({ sourceId: SATELLITE_SOURCE, isSourceLoaded: true, tile: {} }),
    );
    expect(screen.getByRole('status')).not.toHaveTextContent('Loading');
    fireEvent.click(screen.getByRole('button', { name: 'Map' }));
    expect(mock.removeSource).toHaveBeenCalledWith(SATELLITE_SOURCE);
    expect(mock.removeLayer).toHaveBeenCalledWith(SATELLITE_SOURCE);
    act(() =>
      instance.events.error?.({ sourceId: SATELLITE_SOURCE, error: new Error('Late failure') }),
    );
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(() => transform(url)).toThrow();
    expect(mock.maps).toHaveLength(1);
    expect(onViewport).not.toHaveBeenCalled();
  });
  it('returns to the offline map on imagery failure and allows an explicit retry', () => {
    const { instance, onUnavailable } = mount();
    fireEvent.click(screen.getByRole('button', { name: 'Satellite' }));
    act(() => instance.events.error?.({ sourceId: SATELLITE_SOURCE, error: new Error('Offline') }));
    expect(screen.getByRole('status')).toHaveTextContent('Satellite imagery could not be loaded');
    expect(screen.getByRole('button', { name: 'Map' })).toHaveAttribute('aria-pressed', 'true');
    expect(mock.sources.size).toBe(0);
    expect(onUnavailable).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Satellite' }));
    expect(mock.sources.has(SATELLITE_SOURCE)).toBe(true);
    expect(screen.queryByText(/could not be loaded/)).not.toBeInTheDocument();
  });
});
describe('NASA tile request boundary', () => {
  it('rejects other hosts, paths, query strings, invalid coordinates and requests without selection', () => {
    const base = SATELLITE_TILES.tiles![0]!.split('{z}')[0]!;
    expect(satelliteRequest(`${base}8/255/255.jpeg`, true).url).toBe(`${base}8/255/255.jpeg`);
    for (const url of [
      `${base}9/0/0.jpeg`,
      `${base}1/2/0.jpeg`,
      `${base}1/0/2.jpeg`,
      `${base}0/0/0.jpeg?photo=private`,
      `${base}../0/0.jpeg`,
      `${base}0/0/0.jpeg`.replace('nasa.gov', 'example.org'),
    ])
      expect(() => satelliteRequest(url, true)).toThrow();
    expect(() => satelliteRequest(`${base}0/0/0.jpeg`, false)).toThrow();
  });
});
