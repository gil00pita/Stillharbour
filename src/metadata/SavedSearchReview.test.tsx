import { fireEvent, render, screen, within } from '../test/render';
import { expect, it, vi } from 'vitest';
import { defaultQuery } from '../library/model';
import { SavedSearchReview } from './SavedSearchReview';
import type { CatalogCollection } from '../bindings';

it('reviews the exact replacement filters and allows canceling without a write', () => {
  const current: CatalogCollection = {
    id: 'search',
    name: 'Coast',
    kind: 'search',
    version: 2,
    archived: false,
    memberCount: 0,
    query: { ...defaultQuery, tag: 'Coast' },
  };
  const onApply = vi.fn(async () => {});
  const props = { current, roots: [], collections: [current], onApply };
  const { rerender } = render(
    <SavedSearchReview {...props} query={{ ...defaultQuery, name: 'Harbour', minRating: 4 }} />,
  );
  fireEvent.click(
    screen.getByRole('button', { name: 'Replace saved filters with current filters' }),
  );
  expect(
    within(screen.getByRole('region', { name: 'Saved filters' })).getByText('Coast'),
  ).toBeVisible();
  expect(
    within(screen.getByRole('region', { name: 'Replacement filters' })).getByText('Harbour'),
  ).toBeVisible();
  expect(onApply).not.toHaveBeenCalled();
  expect(screen.getByRole('region', { name: 'Review saved search filters' })).toHaveFocus();
  fireEvent.click(screen.getByRole('button', { name: 'Keep saved filters' }));
  expect(
    screen.getByRole('button', { name: 'Replace saved filters with current filters' }),
  ).toHaveFocus();
  expect(onApply).not.toHaveBeenCalled();
  fireEvent.click(
    screen.getByRole('button', { name: 'Replace saved filters with current filters' }),
  );
  rerender(
    <SavedSearchReview {...props} query={{ ...defaultQuery, name: 'Changed behind review' }} />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Replace saved search filters' }));
  expect(onApply).toHaveBeenCalledWith({ ...defaultQuery, name: 'Harbour', minRating: 4 });
});
