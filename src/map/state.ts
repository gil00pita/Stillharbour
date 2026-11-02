import type { AssetQuery, MapBounds, MapCell } from '../bindings';
import { defaultQuery } from '../library/model';

export const WORLD: MapBounds = { west: -180, south: -90, east: 180, north: 90 };
export type MapBrowseState = {
  query: AssetQuery;
  bounds: MapBounds;
  zoom: number;
  cell: MapCell | null;
  selectedId: string | null;
  unlocated: boolean;
};
export function initialMapState(): MapBrowseState {
  return {
    query: { ...defaultQuery },
    bounds: WORLD,
    zoom: 0,
    cell: null,
    selectedId: null,
    unlocated: false,
  };
}
export function mapPhotoQuery(state: MapBrowseState): AssetQuery {
  return {
    ...state.query,
    mapBounds: state.unlocated ? null : state.bounds,
    mapCell: state.unlocated ? null : state.cell,
    location: state.unlocated ? 'unavailable' : 'known',
  };
}
export function normalizeViewport(
  west: number,
  south: number,
  east: number,
  north: number,
): MapBounds {
  const wrap = (value: number) => ((((value + 180) % 360) + 360) % 360) - 180;
  const fullWorld = east - west >= 359.999999;
  return {
    west: fullWorld ? -180 : wrap(west),
    east: fullWorld ? 180 : wrap(east),
    // Mercator cannot display the poles; keep their photos in the edge lists.
    south: south <= -85 ? -90 : Math.max(-90, south),
    north: north >= 85 ? 90 : Math.min(90, north),
  };
}
