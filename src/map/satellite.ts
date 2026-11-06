import type { RasterSourceSpecification, RequestParameters } from 'maplibre-gl';
import provider from '../../assets/maps/satellite-provider.json';

export const SATELLITE_SOURCE = 'nasa-satellite';
export const SATELLITE_CREDIT = provider.attribution;
export const SATELLITE_TILES: RasterSourceSpecification = {
  type: 'raster',
  tiles: [`${provider.tileBase}{z}/{y}/{x}.jpeg`],
  tileSize: provider.tileSize,
  minzoom: 0,
  maxzoom: provider.maxZoom,
  attribution: provider.attribution,
};

// Only compiled NASA tile coordinates are admitted, after an explicit view choice.
export function satelliteRequest(url: string, enabled: boolean): RequestParameters {
  const suffix = url.startsWith(provider.tileBase) ? url.slice(provider.tileBase.length) : '';
  const match = /^(\d+)\/(\d+)\/(\d+)\.jpeg$/.exec(suffix);
  if (!enabled || !match) throw new Error('Map resource is unavailable.');
  const z = Number(match[1]);
  const y = Number(match[2]);
  const x = Number(match[3]);
  if (z > provider.maxZoom || y >= 2 ** z || x >= 2 ** z)
    throw new Error('Map tile coordinates are out of range.');
  return { url, credentials: 'same-origin', cache: 'no-store', referrerPolicy: 'no-referrer' };
}
