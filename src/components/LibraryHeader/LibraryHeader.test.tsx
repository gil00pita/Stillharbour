import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { LibraryHeader } from '@/components/LibraryHeader';

it('keeps Photos focused on filtering and explicit selection, without Catalog or Saved plans', () => {
  const onSelect = vi.fn();
  const onFilter = vi.fn();
  const props = {
    section: 'Photos' as const,
    value: '',
    onSearch: vi.fn(),
    onPlace: vi.fn(),
    busy: false,
    onFilter,
    filterOpen: false,
    activeCount: 2,
    onSelect,
  };
  const view = render(<LibraryHeader {...props} />);
  expect(screen.queryByRole('button', { name: 'Catalog' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Saved plans' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Select' }));
  fireEvent.click(screen.getByRole('button', { name: 'Filter (2 active)' }));
  expect(onSelect).toHaveBeenCalledOnce();
  expect(onFilter).toHaveBeenCalledOnce();
  view.rerender(<LibraryHeader {...props} selecting busy />);
  expect(screen.getByRole('button', { name: 'Done selecting' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Filter (2 active)' })).toBeDisabled();
  expect(screen.getByRole('searchbox')).toBeDisabled();
});

it('shows a removable map scope beside search without adding a filename constraint', () => {
  const clear = vi.fn();
  const onSearch = vi.fn();
  render(
    <LibraryHeader
      section="Photos"
      value=""
      onSearch={onSearch}
      onPlace={vi.fn()}
      busy={false}
      onFilter={vi.fn()}
      filterOpen={false}
      activeCount={1}
      mapScope={{ label: 'Map area', onClear: clear }}
    />,
  );
  const chip = screen.getByRole('button', { name: 'Clear Map area filter' });
  expect(chip.closest('.unified-search')).toBeInTheDocument();
  expect(screen.getByRole('searchbox')).toHaveValue('');
  fireEvent.click(chip);
  expect(clear).toHaveBeenCalledOnce();
  expect(screen.getByRole('searchbox')).toHaveFocus();
  expect(onSearch).not.toHaveBeenCalled();
});
