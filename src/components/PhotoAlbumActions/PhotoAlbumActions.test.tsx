import { useState } from 'react';
import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import type { LibraryStatus } from '../../bindings';
import {
  PhotoAlbumActions,
  usePhotoAlbumMenu,
  usePhotoAlbumKeys,
} from '@/components/PhotoAlbumActions';
import { AssetTile } from '@/components/AssetTile';
import { fixtureAsset } from '../../library/__tests__/fixtures';
import { metadata } from '../../metadata/ipc';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
vi.mock('@/components/AlbumEditor', () => ({
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
  const keys = usePhotoAlbumKeys(asset);
  return (
    <article onContextMenu={context} onKeyDown={keys}>
      <span>Photo area</span>
      <button className="asset-open">Open photo</button>
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
it('uses the context menu to add a photo to an existing custom album without a dots button', async () => {
  const changed = vi.fn(async () => {});
  render(
    <PhotoAlbumActions status={status} onChanged={changed}>
      <Photo />
    </PhotoAlbumActions>,
  );
  fireEvent.contextMenu(screen.getByText('Photo area'), { clientX: 100, clientY: 150 });
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
  const button = screen.getByRole('button', { name: 'Open photo' });
  button.focus();
  fireEvent.keyDown(button, { key: 'F10', shiftKey: true });
  const menu = await screen.findByRole('menu', { name: 'Photo options' });
  await waitFor(() => expect(menu).toHaveFocus());
  fireEvent.keyDown(menu, { key: 'Escape' });
  await waitFor(() => expect(button).toHaveFocus());
  fireEvent.keyDown(button, { key: 'F10', shiftKey: true });
  const again = await screen.findByRole('menu', { name: 'Photo options' });
  await waitFor(() => expect(again).toHaveFocus());
  fireEvent.keyDown(again, { key: 'ArrowDown' });
  await waitFor(() => expect(screen.getByRole('menuitem')).toHaveAttribute('data-highlighted'));
  fireEvent.keyDown(again, { key: 'Enter' });
  expect(await screen.findByRole('dialog', { name: 'Add to album' })).toBeVisible();
});

function SelectablePhoto() {
  const [active, setActive] = useState(false);
  const [selected, setSelected] = useState(false);
  return (
    <PhotoAlbumActions
      status={status}
      onChanged={vi.fn()}
      selection={{
        active,
        disabled: false,
        onSelect: () => {
          setActive(true);
          setSelected(true);
        },
      }}
    >
      <button
        onClick={() => {
          setActive(false);
          setSelected(false);
        }}
      >
        Done selecting
      </button>
      <AssetTile
        asset={asset}
        columns={[]}
        mode="gallery"
        width={200}
        imageHeight={180}
        selected={selected}
        selectionActive={selected}
        frozen={false}
        offline={false}
        edge={256}
        fit="contain"
        preview={undefined}
        onSelect={() => setSelected(!selected)}
        onOpen={vi.fn()}
        onNavigate={vi.fn()}
      />
    </PhotoAlbumActions>
  );
}
it('keeps hover selection available, selects the context photo, and clears it when finished', async () => {
  render(<SelectablePhoto />);
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  const photo = screen.getByRole('button', { name: `Open ${asset.name}` });
  expect(document.querySelector('.photo-album-menu-button')).toBeNull();
  fireEvent.contextMenu(photo, { clientX: 100, clientY: 150 });
  const select = await screen.findByRole('menuitem', { name: 'Select' });
  fireEvent.click(select);
  expect(screen.getByRole('checkbox')).toBeChecked();
  fireEvent.click(photo);
  expect(screen.getByRole('checkbox')).not.toBeChecked();
  fireEvent.click(screen.getByRole('button', { name: 'Done selecting' }));
  expect(screen.getByRole('checkbox')).not.toBeChecked();
});
it('disables context selection while a batch action is busy', async () => {
  const onSelect = vi.fn();
  render(
    <PhotoAlbumActions
      status={status}
      onChanged={vi.fn()}
      selection={{ active: false, disabled: true, onSelect }}
    >
      <Photo />
    </PhotoAlbumActions>,
  );
  fireEvent.contextMenu(screen.getByText('Photo area'));
  const select = await screen.findByRole('menuitem', { name: 'Select' });
  expect(select).toHaveAttribute('aria-disabled', 'true');
  fireEvent.click(select);
  expect(onSelect).not.toHaveBeenCalled();
});
