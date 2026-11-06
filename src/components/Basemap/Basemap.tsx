import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { IconButton } from '@chakra-ui/react';
import {
  Map,
  Marker,
  setWorkerCount,
  setWorkerUrl,
  setMaxParallelImageRequests,
  type GeoJSONSourceSpecification,
  type Subscription,
} from 'maplibre-gl';
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';
import 'maplibre-gl/dist/maplibre-gl.css';
import land from '../../../assets/maps/ne_110m_land.geojson?raw';
import type { MapBounds, MapCluster, ThumbnailResult } from '../../bindings';
import { CachedPreviewImage } from '@/components/CachedPreviewImage';
import { previewKey, previewUrl } from '../../library/previews';
import { normalizeViewport } from '../../map/state';
import { Button } from '@/components/Button';
import {
  SATELLITE_SOURCE,
  SATELLITE_CREDIT,
  SATELLITE_TILES,
  satelliteRequest,
} from '../../map/satellite';

setWorkerCount(1);
setMaxParallelImageRequests(4);
setWorkerUrl(workerUrl);
const landData = JSON.parse(land) as GeoJSONSourceSpecification['data'];

export default function Basemap({
  bounds,
  zoom,
  clusters,
  previews,
  selectedId,
  updating,
  onViewport,
  onCluster,
  onUnavailable,
  placeView,
}: {
  bounds: MapBounds;
  zoom: number;
  clusters: MapCluster[];
  previews: Record<string, ThumbnailResult>;
  selectedId: string | null;
  updating: boolean;
  onViewport: (bounds: MapBounds, zoom: number) => void;
  onCluster: (cluster: MapCluster) => void;
  onUnavailable: () => void;
  placeView: { bounds: MapBounds; zoom: number } | null;
}) {
  const container = useRef<HTMLDivElement>(null);
  const [map, setMap] = useState<Map | null>(null);
  const [view, setView] = useState<'map' | 'satellite'>('map');
  const [satelliteFailed, setSatelliteFailed] = useState(false);
  const [satelliteLoaded, setSatelliteLoaded] = useState(false);
  const satelliteEnabled = useRef(false);
  const callbacks = useRef({ onViewport, onUnavailable });
  useEffect(() => {
    callbacks.current = { onViewport, onUnavailable };
  }, [onViewport, onUnavailable]);
  const initial = useRef({ bounds, zoom });
  useEffect(() => {
    if (!container.current) return;
    const subscriptions: Subscription[] = [];
    let remove: (() => void) | undefined;
    let observer: ResizeObserver | undefined;
    function dispose() {
      satelliteEnabled.current = false;
      observer?.disconnect();
      for (const subscription of subscriptions) subscription.unsubscribe();
      remove?.();
    }
    try {
      const instance = new Map({
        container: container.current,
        style: {
          version: 8,
          sources: { land: { type: 'geojson', data: landData } },
          layers: [
            { id: 'ocean', type: 'background', paint: { 'background-color': '#171f24' } },
            {
              id: 'land',
              type: 'fill',
              source: 'land',
              paint: { 'fill-color': '#344344', 'fill-outline-color': '#5b6a65' },
            },
          ],
        },
        center: [0, 20],
        zoom: initial.current.zoom,
        maxZoom: 16,
        maxTileCacheSize: 64,
        renderWorldCopies: false,
        attributionControl: false,
        dragRotate: false,
        pitchWithRotate: false,
        touchPitch: false,
        // Offline geometry is bundled; only explicitly selected NASA tiles may load.
        transformRequest: (url) => satelliteRequest(url, satelliteEnabled.current),
      });
      remove = () => instance.remove();
      instance
        .getCanvas()
        .setAttribute(
          'aria-label',
          'Photo map. Use arrow keys to pan, plus and minus to zoom. Photos are also listed below.',
        );
      subscriptions.push(
        instance.on('error', (event) => {
          if ('sourceId' in event && event.sourceId === SATELLITE_SOURCE) {
            if (!satelliteEnabled.current) return;
            satelliteEnabled.current = false;
            removeSatellite(instance);
            setSatelliteFailed(true);
            setView('map');
          } else callbacks.current.onUnavailable();
        }),
      );
      subscriptions.push(
        instance.on('sourcedataloading', (event) => {
          if (satelliteEnabled.current && event.sourceId === SATELLITE_SOURCE)
            setSatelliteLoaded(false);
        }),
      );
      subscriptions.push(
        instance.on('sourcedata', (event) => {
          if (
            satelliteEnabled.current &&
            event.sourceId === SATELLITE_SOURCE &&
            event.isSourceLoaded &&
            (event.sourceDataType === 'idle' || Boolean(event.tile))
          )
            setSatelliteLoaded(true);
        }),
      );
      subscriptions.push(
        instance.on('moveend', (event) => {
          // Resize/initial fit also emit moveend. They must not revoke a chosen
          // cluster or the no-location list while the map is mounting again.
          if (!('originalEvent' in event) || !event.originalEvent) return;
          const b = instance.getBounds();
          callbacks.current.onViewport(
            normalizeViewport(b.getWest(), b.getSouth(), b.getEast(), b.getNorth()),
            Math.floor(instance.getZoom()),
          );
        }),
      );
      if (initial.current.zoom > 0) {
        const b = initial.current.bounds;
        instance.fitBounds(
          [
            [b.west, b.south],
            [b.east < b.west ? b.east + 360 : b.east, b.north],
          ],
          { duration: 0, maxZoom: initial.current.zoom },
        );
      }
      subscriptions.push(
        instance.on('load', () => {
          setMap(instance);
          // Initial zoom zero denotes the complete overview, including polar lists.
        }),
      );
      observer = new ResizeObserver(() => instance.resize());
      observer.observe(container.current);
    } catch {
      dispose();
      callbacks.current.onUnavailable();
      return;
    }
    return dispose;
  }, []);
  useEffect(() => {
    if (!map || !placeView) return;
    const { bounds: b, zoom: maximumZoom } = placeView;
    map.fitBounds(
      [
        [b.west, b.south],
        [b.east < b.west ? b.east + 360 : b.east, b.north],
      ],
      { duration: 0, maxZoom: maximumZoom },
    );
  }, [map, placeView]);
  function changeView(next: 'map' | 'satellite') {
    if (!map || next === view) return;
    satelliteEnabled.current = next === 'satellite';
    setSatelliteFailed(false);
    setSatelliteLoaded(false);
    setView(next);
    try {
      if (next === 'map') removeSatellite(map);
      else {
        map.addSource(SATELLITE_SOURCE, SATELLITE_TILES);
        map.addLayer({
          id: SATELLITE_SOURCE,
          type: 'raster',
          source: SATELLITE_SOURCE,
          paint: { 'raster-fade-duration': 0 },
        });
      }
    } catch {
      satelliteEnabled.current = false;
      removeSatellite(map);
      setSatelliteFailed(true);
      setView('map');
    }
  }
  return (
    <div className="map-basemap" ref={container}>
      {map &&
        clusters.map((cluster) => (
          <PhotoMarker
            key={cluster.key}
            map={map}
            disabled={updating}
            cluster={cluster}
            previews={previews}
            selected={selectedId === cluster.representative.id}
            onClick={() => onCluster(cluster)}
          />
        ))}
      <p className="map-attribution">
        {view === 'satellite'
          ? SATELLITE_CREDIT
          : 'Made with Natural Earth · Offline world overview'}
      </p>
      {map && (
        <div className="map-layers" role="group" aria-label="Map view">
          <Button type="button" aria-pressed={view === 'map'} onClick={() => changeView('map')}>
            Map
          </Button>
          <Button
            type="button"
            aria-pressed={view === 'satellite'}
            title="NASA satellite overview · limited detail · internet required; sends viewed map areas to NASA"
            onClick={() => changeView('satellite')}
          >
            Satellite
          </Button>
        </div>
      )}
      {(view === 'satellite' || satelliteFailed) && (
        <p className="map-view-note" role="status">
          {satelliteFailed
            ? 'Satellite imagery could not be loaded. Map view is available; check your connection and try Satellite again.'
            : satelliteLoaded
              ? 'NASA satellite overview · limited detail · internet required'
              : 'Loading NASA satellite overview · limited detail · internet required'}
        </p>
      )}
      {map && (
        <div className="map-zoom" role="group" aria-label="Map zoom">
          <button
            aria-label="Zoom in"
            disabled={zoom >= 16}
            onClick={(event) => map.zoomIn({ duration: 0 }, { originalEvent: event.nativeEvent })}
          >
            +
          </button>
          <button
            aria-label="Zoom out"
            disabled={zoom <= 0}
            onClick={(event) => map.zoomOut({ duration: 0 }, { originalEvent: event.nativeEvent })}
          >
            −
          </button>
        </div>
      )}
      {map && (
        <IconButton
          className="map-world"
          aria-label="Whole world"
          title="Show whole world"
          variant="outline"
          onClick={() => {
            map.jumpTo({ center: [0, 20], zoom: 0 });
            onViewport({ west: -180, south: -90, east: 180, north: 90 }, 0);
          }}
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="9" />
            <ellipse cx="12" cy="12" rx="4" ry="9" />
            <path d="M3 12h18M5 6.5h14M5 17.5h14" />
          </svg>
        </IconButton>
      )}
    </div>
  );
}

function removeSatellite(map: Map) {
  if (map.getLayer(SATELLITE_SOURCE)) map.removeLayer(SATELLITE_SOURCE);
  if (map.getSource(SATELLITE_SOURCE)) map.removeSource(SATELLITE_SOURCE);
}

function PhotoMarker({
  map,
  cluster,
  previews,
  selected,
  disabled,
  onClick,
}: {
  map: Map;
  cluster: MapCluster;
  previews: Record<string, ThumbnailResult>;
  selected: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  const [element] = useState(() => document.createElement('div'));
  useEffect(() => {
    const marker = new Marker({ element })
      .setLngLat([
        cluster.longitude,
        Math.max(-85.05112878, Math.min(85.05112878, cluster.latitude)),
      ])
      .addTo(map);
    return () => {
      marker.remove();
    };
  }, [map, element, cluster.longitude, cluster.latitude]);
  const asset = cluster.representative;
  const preview = previews[previewKey({ assetId: asset.id, revision: asset.revision, edge: 256 })];
  const edge = preview?.state === 'ready' ? 256 : preview?.fallbackEdge;
  return createPortal(
    <button
      className={`map-marker${selected ? ' selected' : ''}`}
      aria-pressed={selected}
      disabled={disabled}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        onClick();
      }}
      aria-label={`${cluster.count} ${cluster.count === 1 ? 'photo' : 'photos'} in this area; sample location ${cluster.latitude.toFixed(3)}, ${cluster.longitude.toFixed(3)}`}
    >
      <span className="map-marker-image">
        {edge ? (
          <CachedPreviewImage src={previewUrl(asset, edge)} alt="" />
        ) : (
          <span aria-hidden="true">◇</span>
        )}
      </span>
      <span className="map-marker-count">{cluster.count}</span>
    </button>,
    element,
  );
}
