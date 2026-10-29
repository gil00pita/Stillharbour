import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { DirectoryPage, LibraryStatus } from '../bindings';
import { FolderExplorer } from './FolderExplorer';
import { fixtureAsset } from './__tests__/fixtures';
import { libraryData } from '../ipc';
import type { ExplorerItem } from './ExplorerCanvas';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
// The canvas below is deliberately structural; native tests cover focus/scroll
// restoration. Do not leave real retry timers looking for its absent asset DOM.
vi.mock('./focus', () => ({ restoreAssetFocus: vi.fn() }));
vi.mock('./ExplorerCanvas', () => ({
  ExplorerCanvas: ({
    items,
    onOpen,
  }: {
    items: ExplorerItem[];
    onOpen: (item: ExplorerItem, button: HTMLButtonElement) => void;
  }) => (
    <div>
      {items.map((item) => (
        <button
          key={
            item.kind === 'root'
              ? item.root.id
              : item.kind === 'folder'
                ? item.directory.id
                : item.asset.id
          }
          onClick={(event) => onOpen(item, event.currentTarget)}
        >
          {item.kind === 'root'
            ? item.root.label
            : item.kind === 'folder'
              ? item.directory.name
              : item.asset.name}
        </button>
      ))}
    </div>
  ),
}));
vi.mock('./PhotoViewer', () => ({
  PhotoViewer: ({ query, onClose }: { query: unknown; onClose: () => void }) => (
    <div role="dialog" aria-label="Photo viewer">
      <output>{JSON.stringify(query)}</output>
      <button onClick={onClose}>Close viewer</button>
    </div>
  ),
}));
const status: LibraryStatus = {
  roots: [
    {
      id: 'root',
      label: 'Generated library',
      displayPath: 'Generated path',
      mode: 'referenced',
      isBase: true,
      availability: 'online',
      filter: {
        version: 1,
        includeGlobs: [],
        excludeGlobs: [],
        excludedFolders: [],
        extensions: [],
        includeHiddenFiles: false,
        includeHiddenFolders: false,
        minBytes: null,
        maxBytes: null,
        fromDate: null,
        throughDate: null,
      },
      lastScan: null,
      fileCount: 2,
    },
  ],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
function page(directory = 'd'): DirectoryPage {
  return {
    directory: {
      id: directory,
      name: directory === 'd' ? 'Generated library' : 'Trips',
      state: 'complete',
    },
    ancestors: directory === 'd' ? [] : [{ id: 'd', name: 'Generated library', state: 'complete' }],
    entries:
      directory === 'd'
        ? [
            { kind: 'folder', directory: { id: 'd7472697073', name: 'Trips', state: 'complete' } },
            { kind: 'file', asset: { ...fixtureAsset('note'), name: 'notes.txt' }, media: false },
          ]
        : [{ kind: 'file', asset: { ...fixtureAsset('photo'), name: 'trip.jpg' }, media: true }],
    total: directory === 'd' ? 2 : 1,
    previousCursor: null,
    nextCursor: null,
    revision: '1',
  };
}
beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'browse_directory') return page(request.directory_id ?? 'd');
      throw new Error('Unexpected request');
    });
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = false;
    },
  });
});
function setup() {
  render(<FolderExplorer status={status} onError={vi.fn()} onRefresh={vi.fn(async () => {})} />);
}
it('starts with libraries and opens only immediate children, with breadcrumbs and Back', async () => {
  setup();
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Generated library' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Trips' }));
  expect(await screen.findByRole('button', { name: 'trip.jpg' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'notes.txt' })).not.toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledWith(
    expect.objectContaining({
      kind: 'browse_directory',
      root_id: 'root',
      directory_id: 'd7472697073',
    }),
    'directory',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Back' }));
  expect(await screen.findByRole('button', { name: 'notes.txt' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'trip.jpg' })).not.toBeInTheDocument();
});
it('constrains media viewer navigation to the exact native directory', async () => {
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Generated library' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Trips' }));
  fireEvent.click(await screen.findByRole('button', { name: 'trip.jpg' }));
  expect(screen.getByRole('dialog', { name: 'Photo viewer' })).toHaveTextContent(
    '"directoryId":"d7472697073"',
  );
  expect(screen.getByRole('dialog')).toHaveTextContent('"rootId":"root"');
});
it('shows non-media catalog details without opening an external application', async () => {
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Generated library' }));
  fireEvent.click(await screen.findByRole('button', { name: 'notes.txt' }));
  expect(screen.getByRole('dialog', { name: 'File details' })).toHaveTextContent(
    'does not open external applications',
  );
  expect(libraryData).toHaveBeenCalledTimes(1);
});
it('keeps folder read failures explicit and retries instead of calling the folder empty', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Catalog unavailable'));
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Generated library' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  expect(screen.getByRole('heading', { name: 'Folder could not be loaded' })).toBeVisible();
  expect(screen.queryByRole('heading', { name: 'No indexed items here' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Retry folder' }));
  expect(await screen.findByRole('button', { name: 'Trips' })).toBeVisible();
});
it('ignores a late folder response after navigating back to Libraries', async () => {
  let resolve!: (value: DirectoryPage) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Generated library' }));
  fireEvent.click(screen.getByRole('button', { name: 'Up' }));
  resolve(page());
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Generated library' })).toBeVisible(),
  );
  expect(screen.queryByRole('button', { name: 'Trips' })).not.toBeInTheDocument();
});
