import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import {
  Map,
  Marker,
  setWorkerCount,
  setWorkerUrl,
  type GeoJSONSourceSpecification,
} from 'maplibre-gl';
import workerUrl from 'maplibre-gl/dist/maplibre-gl-worker.mjs?worker&url';
import 'maplibre-gl/dist/maplibre-gl.css';
import land from '../../assets/maps/ne_110m_land.geojson?raw';
import type { MapBounds, MapCluster, ThumbnailResult } from '../bindings';
import { CachedPreviewImage } from '../library/CachedPreviewImage';
import { previewKey, previewUrl } from '../library/previews';
import { normalizeViewport } from './state';

setWorkerCount(1);
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
  const callbacks = useRef({ onViewport, onUnavailable });
  useEffect(() => {
    callbacks.current = { onViewport, onUnavailable };
  }, [onViewport, onUnavailable]);
  const initial = useRef({ bounds, zoom });
  useEffect(() => {
    if (!container.current) return;
    let instance: Map;
    try {
      instance = new Map({
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
        renderWorldCopies: false,
        attributionControl: false,
        dragRotate: false,
        pitchWithRotate: false,
        touchPitch: false,
        // All geometry is bundled inline. No external style, tiles, glyphs or sprites.
        transformRequest: () => {
          throw new Error('External map resources are unavailable.');
        },
      });
      instance
        .getCanvas()
        .setAttribute(
          'aria-label',
          'Photo map. Use arrow keys to pan, plus and minus to zoom. Photos are also listed below.',
        );
      instance.on('error', () => callbacks.current.onUnavailable());
      instance.on('moveend', (event) => {
        // Resize/initial fit also emit moveend. They must not revoke a chosen
        // cluster or the no-location list while the map is mounting again.
        if (!('originalEvent' in event) || !event.originalEvent) return;
        const b = instance.getBounds();
        callbacks.current.onViewport(
          normalizeViewport(b.getWest(), b.getSouth(), b.getEast(), b.getNorth()),
          Math.floor(instance.getZoom()),
        );
      });
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
      instance.on('load', () => {
        setMap(instance);
        // Initial zoom zero denotes the complete overview, including polar lists.
      });
    } catch {
      callbacks.current.onUnavailable();
      return;
    }
    const observer = new ResizeObserver(() => instance.resize());
    observer.observe(container.current);
    return () => {
      observer.disconnect();
      instance.remove();
    };
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
      <p className="map-attribution">Made with Natural Earth · Offline world overview</p>
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
        <button
          className="map-world"
          onClick={() => {
            map.jumpTo({ center: [0, 20], zoom: 0 });
            onViewport({ west: -180, south: -90, east: 180, north: 90 }, 0);
          }}
        >
          Whole world
        </button>
      )}
    </div>
  );
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
