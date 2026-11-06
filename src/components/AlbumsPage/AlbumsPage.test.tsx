import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import type { AlbumPage, LibraryStatus } from '../../bindings';
import { defaultQuery } from '../../library/model';
import { AlbumsPage } from '@/components/AlbumsPage';
import { metadata } from '../../metadata/ipc';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
vi.mock('../../library/previews', () => ({ usePreviews: () => ({ results: {} }) }));
vi.mock('@/components/AlbumEditor', () => ({
  AlbumEditor: () => <div role="dialog">Create an album</div>,
}));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const data: AlbumPage = {
  items: [
    {
      id: 'tag:Coast',
      name: 'Coast',
      fromTags: true,
      version: 0,
      photoCount: 2,
      covers: [],
      query: { ...defaultQuery, tag: 'Coast' },
    },
    {
      id: 'custom',
      name: 'Weekends',
      fromTags: false,
      version: 1,
      photoCount: 0,
      covers: [],
      query: { ...defaultQuery, albumId: 'custom' },
    },
  ],
  total: 2,
  nextOffset: null,
};
beforeEach(() => {
  vi.mocked(metadata).mockReset().mockResolvedValue(data);
  vi.stubGlobal(
    'IntersectionObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
});
it('shows custom and tag albums together, opens albums and offers creation without filter buttons', async () => {
  const onOpen = vi.fn();
  render(<AlbumsPage onBrowse={vi.fn()} status={status} onOpen={onOpen} onRefresh={vi.fn()} />);
  expect(await screen.findByText('From tags')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Catalog' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Saved plans' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Weekends, 0 photos' })).toBeVisible();
  expect(
    screen.queryByRole('button', { name: /all albums|custom albums|tag albums/i }),
  ).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: /Coast, 2 photos/ }));
  expect(onOpen).toHaveBeenCalledWith(data.items[0]);
  fireEvent.click(screen.getByRole('button', { name: /New album/ }));
  expect(screen.getByRole('dialog')).toBeVisible();
});
it('searches album names and gives a useful empty state', async () => {
  render(<AlbumsPage onBrowse={vi.fn()} status={status} onOpen={vi.fn()} onRefresh={vi.fn()} />);
  await screen.findByText('From tags');
  vi.mocked(metadata).mockResolvedValue({ items: [], total: 0, nextOffset: null });
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search albums' }), {
    target: { value: 'missing' },
  });
  expect(await screen.findByText('No albums found')).toBeVisible();
  expect(metadata).toHaveBeenLastCalledWith(
    { kind: 'albums', search: 'missing', offset: 0 },
    'albums',
  );
});
it('exposes loading failures with a working retry', async () => {
  vi.mocked(metadata).mockRejectedValueOnce(new Error('Catalog unavailable'));
  render(<AlbumsPage onBrowse={vi.fn()} status={status} onOpen={vi.fn()} onRefresh={vi.fn()} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  fireEvent.click(screen.getByText('Retry albums'));
  await waitFor(() => expect(screen.getByText('From tags')).toBeVisible());
});
