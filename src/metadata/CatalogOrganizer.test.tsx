import { fireEvent, render, screen, waitFor } from '../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { CatalogCollection, CatalogMetadataRequest } from '../bindings';
import { defaultQuery } from '../library/model';
import { CatalogOrganizer } from './CatalogOrganizer';
import { metadata } from './ipc';
vi.mock('./ipc', () => ({ metadata: vi.fn() }));
const album: CatalogCollection = {
  id: 'album',
  version: 2,
  kind: 'album',
  name: 'Generated album',
  query: null,
  archived: false,
  memberCount: 2,
};
const browse = vi.fn();
const close = vi.fn();
const changed = vi.fn(async () => {});
function renderPanel() {
  return render(
    <CatalogOrganizer
      query={{ ...defaultQuery, rootId: 'root', fromDate: '2026-01-01' }}
      selection={{ kind: 'ids', ids: ['photo-1', 'photo-2'] }}
      selectionCount={2}
      onBrowse={browse}
      onClose={close}
      onChanged={changed}
    />,
  );
}
beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.setAttribute('open', '');
  };
  HTMLDialogElement.prototype.close = function () {
    this.removeAttribute('open');
  };
  browse.mockReset();
  close.mockReset();
  changed.mockClear();
  vi.mocked(metadata)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'collections') return [album] as never;
      if (request.kind === 'tag_categories') return [] as never;
      if (request.kind === 'collection_members') return { collection: album, changed: 2 } as never;
      return album as never;
    });
});
it('creates a saved search from the current filter data and never sends executable authority', async () => {
  renderPanel();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Create collection' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByLabelText('New collection name'), {
    target: { value: '2026 photos' },
  });
  fireEvent.change(screen.getByLabelText('Collection type'), { target: { value: 'search' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create collection' }));
  await waitFor(() =>
    expect(metadata).toHaveBeenCalledWith(
      {
        kind: 'create_collection',
        collection_kind: 'search',
        name: '2026 photos',
        query: expect.objectContaining({ rootId: 'root', fromDate: '2026-01-01' }),
      },
      'collection',
    ),
  );
});
it('adds the exact selection and count using the displayed collection version', async () => {
  renderPanel();
  await screen.findByRole('option', { name: /Generated album/ });
  fireEvent.change(screen.getByLabelText('Catalog collection'), { target: { value: 'album' } });
  fireEvent.click(screen.getByRole('button', { name: 'Add selection to album' }));
  await waitFor(() =>
    expect(metadata).toHaveBeenCalledWith(
      {
        kind: 'collection_members',
        id: 'album',
        expected_version: 2,
        expected_count: 2,
        selection: { kind: 'ids', ids: ['photo-1', 'photo-2'] },
        remove: false,
      },
      'membership',
    ),
  );
});
it('applies album membership as a shared native library filter', async () => {
  renderPanel();
  await screen.findByRole('option', { name: /Generated album/ });
  fireEvent.change(screen.getByLabelText('Catalog collection'), { target: { value: 'album' } });
  fireEvent.click(screen.getByRole('button', { name: 'View album' }));
  expect(browse).toHaveBeenCalledWith(expect.objectContaining({ albumId: 'album' }));
  expect(close).toHaveBeenCalledOnce();
});
it('keeps organization drafts and errors when the backend refuses a stale save', async () => {
  renderPanel();
  await screen.findByRole('option', { name: /Generated album/ });
  fireEvent.change(screen.getByLabelText('Catalog collection'), { target: { value: 'album' } });
  fireEvent.change(screen.getByLabelText('Collection name'), { target: { value: 'My new name' } });
  vi.mocked(metadata).mockImplementation(async (request: CatalogMetadataRequest) => {
    if (request.kind === 'update_collection') throw new Error('Catalog metadata changed.');
    return (request.kind === 'collections' ? [album] : []) as never;
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save collection name' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog metadata changed.');
  expect(screen.getByLabelText('Collection name')).toHaveValue('My new name');
});
it('does not call a failed list an empty catalog', async () => {
  vi.mocked(metadata).mockRejectedValueOnce(new Error('Catalog unavailable.'));
  renderPanel();
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable.');
  expect(screen.queryByText('No albums or saved searches yet.')).not.toBeInTheDocument();
});
