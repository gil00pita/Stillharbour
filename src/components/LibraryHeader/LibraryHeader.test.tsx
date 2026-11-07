import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { LibraryHeader } from '@/components/LibraryHeader';

it('changes thumbnail scale without changing search or selection', () => {
  const onThumbnailSize = vi.fn();
  const onSearch = vi.fn();
  const onSelect = vi.fn();
  render(
    <LibraryHeader
      section="Photos"
      value="Summer"
      onSearch={onSearch}
      onPlace={vi.fn()}
      busy={false}
      onFilter={vi.fn()}
      filterOpen={false}
      activeCount={0}
      onSelect={onSelect}
      thumbnailSize="comfortable"
      onThumbnailSize={onThumbnailSize}
    />,
  );
  fireEvent.change(screen.getByLabelText('Thumbnail scale'), { target: { value: '2' } });
  expect(onThumbnailSize).toHaveBeenCalledWith('large');
  expect(screen.getByRole('searchbox')).toHaveValue('Summer');
  expect(onSelect).not.toHaveBeenCalled();
  expect(onSearch).not.toHaveBeenCalled();
});

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
  const select = screen.getByRole('button', { name: 'Select', pressed: false });
  expect(select).toHaveAttribute('title', 'Select');
  fireEvent.click(select);
  fireEvent.click(screen.getByRole('button', { name: 'Filter (2 active)' }));
  expect(onSelect).toHaveBeenCalledOnce();
  expect(onFilter).toHaveBeenCalledOnce();
  view.rerender(<LibraryHeader {...props} selecting />);
  const done = screen.getByRole('button', { name: 'Done selecting', pressed: true });
  expect(done).toHaveAttribute('title', 'Done selecting');
  fireEvent.click(done);
  expect(onSelect).toHaveBeenCalledTimes(2);
  view.rerender(<LibraryHeader {...props} selecting busy />);
  expect(screen.getByRole('button', { name: 'Done selecting' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Filter (2 active)' })).toBeDisabled();
  expect(screen.getByRole('searchbox')).toBeDisabled();
});

it.each(['Map area', 'Selected map location'])(
  'embeds %s as a tag with a remove control inside search',
  (label) => {
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
        mapScope={{ label, onClear: clear }}
      />,
    );
    const chip = screen.getByRole('button', { name: `Clear ${label} filter` });
    const frame = chip.closest('.search-input-frame');
    expect(frame).toContainElement(screen.getByRole('searchbox'));
    expect(chip.closest('.search-scope-tag')).toHaveTextContent(label);
    expect(chip).not.toHaveTextContent('Clear');
    expect(chip.querySelector('svg')).toHaveAttribute('aria-hidden', 'true');
    fireEvent.click(screen.getByText(label));
    expect(clear).not.toHaveBeenCalled();
    expect(screen.getByRole('searchbox')).toHaveValue('');
    fireEvent.click(chip);
    expect(clear).toHaveBeenCalledOnce();
    expect(screen.getByRole('searchbox')).toHaveFocus();
    expect(onSearch).not.toHaveBeenCalled();
  },
);
