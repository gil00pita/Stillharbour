import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import type { LibraryStatus } from '../../bindings';
import { PhotoAlbumActions, PhotoAlbumButton, usePhotoAlbumMenu } from './index';
import { fixtureAsset } from '../../library/__tests__/fixtures';
import { metadata } from '../../metadata/ipc';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
vi.mock('../AlbumEditor', () => ({
  AlbumEditor: ({ initialIds }: { initialIds: string[] }) => (
    <div role="dialog">Preselected {initialIds.join(',')}</div>
  ),
}));
const asset = fixtureAsset('photo-a');
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
function Photo() {
  const context = usePhotoAlbumMenu(asset);
  return (
    <article onContextMenu={context}>
      <span>Photo area</span>
      <PhotoAlbumButton asset={asset} />
    </article>
  );
}
beforeEach(() => {
  vi.mocked(metadata)
    .mockReset()
    .mockResolvedValue([
      {
        id: 'custom',
        name: 'Weekends',
        version: 3,
        kind: 'album',
        archived: false,
        query: null,
        memberCount: 5,
      },
    ]);
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
      unobserve() {}
    },
  );
  Element.prototype.scrollIntoView = vi.fn();
});
it('uses the accessible menu button to add a photo to an existing custom album', async () => {
  const changed = vi.fn(async () => {});
  render(
    <PhotoAlbumActions status={status} onChanged={changed}>
      <Photo />
    </PhotoAlbumActions>,
  );
  fireEvent.click(screen.getByRole('button', { name: `Options for ${asset.name}` }));
  const item = await screen.findByRole('menuitem', { name: 'Add to album' });
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Photo options' })).toHaveFocus());
  fireEvent.pointerDown(item);
  fireEvent.click(item);
  fireEvent.click(await screen.findByRole('button', { name: /Weekends/ }));
  await waitFor(() => expect(changed).toHaveBeenCalledOnce());
  expect(metadata).toHaveBeenCalledWith(
    {
      kind: 'collection_members',
      id: 'custom',
      expected_version: 3,
      expected_count: 1,
      selection: { kind: 'ids', ids: ['photo-a'] },
      remove: false,
    },
    'membership',
  );
});
it('opens the right-click menu and creates an album with the photo preselected', async () => {
  render(
    <PhotoAlbumActions status={status} onChanged={vi.fn()}>
      <Photo />
    </PhotoAlbumActions>,
  );
  fireEvent.contextMenu(screen.getByText('Photo area'), { clientX: 100, clientY: 150 });
  const item = await screen.findByRole('menuitem', { name: 'Add to album' });
  await waitFor(() => expect(screen.getByRole('menu', { name: 'Photo options' })).toHaveFocus());
  fireEvent.pointerDown(item);
  fireEvent.click(item);
  fireEvent.click(await screen.findByRole('button', { name: /Create new album/ }));
  expect(screen.getByRole('dialog')).toHaveTextContent('Preselected photo-a');
});

it('restores focus on Escape and supports keyboard menu selection', async () => {
  render(
    <PhotoAlbumActions status={status} onChanged={vi.fn()}>
      <Photo />
    </PhotoAlbumActions>,
  );
  const button = screen.getByRole('button', { name: `Options for ${asset.name}` });
  button.focus();
  fireEvent.click(button);
  const menu = await screen.findByRole('menu', { name: 'Photo options' });
  await waitFor(() => expect(menu).toHaveFocus());
  fireEvent.keyDown(menu, { key: 'Escape' });
  await waitFor(() => expect(button).toHaveFocus());
  fireEvent.click(button);
  const again = await screen.findByRole('menu', { name: 'Photo options' });
  await waitFor(() => expect(again).toHaveFocus());
  fireEvent.keyDown(again, { key: 'ArrowDown' });
  await waitFor(() => expect(screen.getByRole('menuitem')).toHaveAttribute('data-highlighted'));
  fireEvent.keyDown(again, { key: 'Enter' });
  expect(await screen.findByRole('dialog', { name: 'Add to album' })).toBeVisible();
});
