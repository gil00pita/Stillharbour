import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../test/render';
import { LibraryHeader } from './LibraryHeader';

it('keeps secondary actions named and operable when their visible labels collapse', () => {
  const onCatalog = vi.fn();
  const onPlans = vi.fn();
  const onFilter = vi.fn();
  const view = render(
    <LibraryHeader
      section="Photos"
      value=""
      onSearch={vi.fn()}
      onPlace={vi.fn()}
      busy={false}
      onFilter={onFilter}
      filterOpen={false}
      activeCount={2}
      onCatalog={onCatalog}
      onPlans={onPlans}
    />,
  );
  expect(
    screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }),
  ).toHaveAttribute('title', 'Search a filename, DD-MM-YYYY / YYYY-MM-DD, city or country');
  expect(
    screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }),
  ).toBeEnabled();
  fireEvent.click(screen.getByRole('button', { name: 'Catalog' }));
  fireEvent.click(screen.getByRole('button', { name: 'Saved plans' }));
  fireEvent.click(screen.getByRole('button', { name: 'Filter (2 active)' }));
  expect(onCatalog).toHaveBeenCalledOnce();
  expect(onPlans).toHaveBeenCalledOnce();
  expect(onFilter).toHaveBeenCalledOnce();
  expect(screen.getByRole('button', { name: 'Catalog' })).toHaveAttribute('title', 'Catalog');
  expect(screen.getByRole('button', { name: 'Saved plans' })).toHaveAttribute(
    'title',
    'Saved plans',
  );
  view.rerender(
    <LibraryHeader
      section="Photos"
      value=""
      onSearch={vi.fn()}
      onPlace={vi.fn()}
      busy
      onFilter={onFilter}
      filterOpen={false}
      activeCount={2}
      onCatalog={onCatalog}
      onPlans={onPlans}
    />,
  );
  expect(
    screen.getByRole('searchbox', { name: 'Search filenames, dates or places' }),
  ).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Catalog' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Saved plans' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Filter (2 active)' })).toBeDisabled();
});
