import type { AssetQuery } from '../bindings';

export function mapFilterLabel(query: AssetQuery): string | null {
  if (query.location === 'unavailable') return 'No location';
  if (query.mapCell) return 'Selected map location';
  if (query.mapBounds) return 'Map area';
  if (query.location === 'known') return 'With location';
  return null;
}
