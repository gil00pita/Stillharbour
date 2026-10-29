import { fireEvent, render, screen, within } from '@testing-library/react';
import { useState } from 'react';
import { expect, it, vi } from 'vitest';
import { TextListControls, TextListHeader, TextListRow } from './TextList';
import { DEFAULT_TEXT_COLUMNS, TEXT_COLUMNS } from './textColumns';
import { fixtureAsset } from './__tests__/fixtures';

const callbacks = () => ({
  selected: false,
  frozen: false,
  selectionActive: false,
  offline: false,
  onSelect: vi.fn(),
  onOpen: vi.fn(),
  onNavigate: vi.fn(),
});

it('shows the actual modification date without repeated provenance, never substituting capture time', () => {
  const asset = {
    ...fixtureAsset('1', '2001-01-02T03:04:05'),
    modifiedAt: '2026-10-28T10:30:00Z',
    dateSource: 'filesystem_modified',
  };
  const { container, rerender } = render(
    <TextListRow asset={asset} columns={DEFAULT_TEXT_COLUMNS} {...callbacks()} />,
  );
  expect(screen.getByText('2026-10-28 10:30:00')).toBeInTheDocument();
  expect(container.querySelector('[data-column="modified"]')).toHaveAttribute(
    'title',
    asset.modifiedAt,
  );
  expect(screen.queryByText('filesystem modified')).not.toBeInTheDocument();
  expect(screen.queryByText('2001-01-02 03:04:05')).not.toBeInTheDocument();
  rerender(
    <TextListRow
      asset={{ ...asset, modifiedAt: null }}
      columns={DEFAULT_TEXT_COLUMNS}
      {...callbacks()}
    />,
  );
  expect(container.querySelector('[data-column="modified"]')).toHaveTextContent('Unknown');
});

it('lets readers choose other catalog fields, retain Name and reset columns without changing sort', () => {
  const onSort = vi.fn();
  function Controls() {
    const [columns, setColumns] = useState([...DEFAULT_TEXT_COLUMNS]);
    return (
      <>
        <TextListControls
          columns={columns}
          onColumns={setColumns}
          sort="capture"
          onSort={onSort}
          busy={false}
        />
        <TextListHeader columns={columns} sort="capture" busy={false} />
      </>
    );
  }
  render(<Controls />);
  const summary = screen.getByText('Columns');
  fireEvent.click(summary);
  expect(screen.getByRole('checkbox', { name: 'Name (always shown)' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Modified Date' }));
  fireEvent.click(screen.getByRole('checkbox', { name: 'Capture Date' }));
  expect(screen.queryByRole('columnheader', { name: 'Modified Date' })).not.toBeInTheDocument();
  expect(screen.getByRole('columnheader', { name: 'Capture Date' })).toHaveAttribute(
    'aria-sort',
    'descending',
  );
  expect(onSort).not.toHaveBeenCalled();
  fireEvent.change(screen.getByRole('combobox', { name: 'Text list sort' }), {
    target: { value: 'modified' },
  });
  expect(onSort).toHaveBeenCalledWith('modified');
  fireEvent.click(screen.getByRole('button', { name: 'Reset columns' }));
  expect(screen.getByRole('columnheader', { name: 'Modified Date' })).toBeInTheDocument();
  expect(screen.queryByRole('columnheader', { name: 'Capture Date' })).not.toBeInTheDocument();
  summary.focus();
  fireEvent.keyDown(document, { key: 'Escape' });
  expect(summary.closest('details')).not.toHaveAttribute('open');
  expect(summary).toHaveFocus();
});

it('sorts supported column headers through the native query and leaves non-sortable fields as labels', () => {
  const onSort = vi.fn();
  const { rerender } = render(
    <TextListHeader columns={DEFAULT_TEXT_COLUMNS} sort="modified" onSort={onSort} busy={false} />,
  );
  expect(screen.getByRole('columnheader', { name: 'Modified Date' })).toHaveAttribute(
    'aria-sort',
    'descending',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Modified Date' }));
  expect(onSort).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Name' }));
  expect(onSort).toHaveBeenCalledWith('name');
  expect(
    within(screen.getByRole('columnheader', { name: 'Dimensions' })).queryByRole('button'),
  ).not.toBeInTheDocument();
  rerender(<TextListHeader columns={DEFAULT_TEXT_COLUMNS} sort="name" onSort={onSort} busy />);
  expect(screen.getByRole('columnheader', { name: 'Name' })).toHaveAttribute(
    'aria-sort',
    'ascending',
  );
  expect(screen.getByRole('button', { name: 'Size' })).toBeDisabled();
});

it('renders optional metadata and explicit offline status without images, retaining selection and open behavior', () => {
  const c = callbacks();
  const asset = {
    ...fixtureAsset('2'),
    modifiedAt: null,
    camera: 'Fixture camera',
    tags: ['Travel'],
    rating: 4,
  };
  const columns = TEXT_COLUMNS.map((column) => column.id);
  const { container, rerender } = render(
    <TextListRow {...c} asset={asset} columns={columns} offline />,
  );
  expect(screen.getByText('Offline original')).toBeInTheDocument();
  expect(screen.getByText('exif original')).toBeInTheDocument();
  expect(screen.getByText('Fixture camera')).toBeInTheDocument();
  expect(screen.getByText('Travel')).toBeInTheDocument();
  expect(screen.getByText('4 / 5')).toBeInTheDocument();
  expect(container.querySelector('img')).toBeNull();
  const opener = screen.getByRole('button', { name: 'Open Generated 2.jpg' });
  fireEvent.click(opener);
  expect(c.onOpen).toHaveBeenCalledWith(asset.id, opener);
  rerender(<TextListRow {...c} asset={asset} columns={columns} selected selectionActive />);
  fireEvent.click(screen.getByRole('button', { name: 'Open Generated 2.jpg' }), { shiftKey: true });
  expect(c.onSelect).toHaveBeenCalledWith(asset.id, true);
  expect(screen.getByRole('checkbox', { name: 'Select Generated 2.jpg' })).toBeChecked();
});
