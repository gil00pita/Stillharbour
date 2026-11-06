import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { ActiveFilters } from '@/components/FilterFields';
import { defaultQuery } from '../../library/model';
import { WORLD } from '../../map/state';
import type { AssetQuery } from '../../bindings';

it.each<[Partial<AssetQuery>, string]>([
  [{ mapBounds: WORLD, location: 'known' }, 'Map area'],
  [
    { mapBounds: WORLD, mapCell: { zoom: 2, x: 1, y: 1 }, location: 'known' },
    'Selected map location',
  ],
  [{ location: 'unavailable' }, 'No location'],
  [{ location: 'known' }, 'With location'],
])('exposes %s in active filters and clears its complete spatial constraint', (scope, label) => {
  const filter = vi.fn();
  render(
    <ActiveFilters
      query={{ ...defaultQuery, ...scope, name: 'flowers', placeId: 'city' }}
      filter={filter}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: `Remove ${label} filter` }));
  expect(filter).toHaveBeenCalledWith({ mapBounds: null, mapCell: null, location: null });
  expect(filter.mock.calls[0]?.[0]).not.toHaveProperty('name');
  expect(filter.mock.calls[0]?.[0]).not.toHaveProperty('placeId');
});
