import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import type { LibraryStatus } from '../bindings';
import { ComparisonViewer } from './ComparisonViewer';
import { fixtureAsset } from './__tests__/fixtures';
import { defaultFilter } from './model';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: () => 'thumbnail://cache/' }));
const assets = [fixtureAsset('a'), fixtureAsset('b')];
const status: LibraryStatus = {
  roots: [
    {
      id: 'generated-root',
      label: 'Generated fixture',
      displayPath: 'Generated fixture',
      mode: 'referenced',
      isBase: true,
      availability: 'offline',
      filter: defaultFilter,
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
beforeEach(() => {
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
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'thumbnails')
        return request.keys.map((key) => ({ ...key, state: 'ready', error: null }));
      throw new Error('Unexpected request');
    });
});
it('distinguishes an offline original library from catalog presence while showing cached previews', async () => {
  render(<ComparisonViewer assets={assets} status={status} onClose={vi.fn()} onError={vi.fn()} />);
  expect(await screen.findByRole('img', { name: assets[0]!.name })).toBeVisible();
  expect(screen.getAllByText(/Catalog: present · Original library: offline/)).toHaveLength(2);
  expect(screen.getByText(/does not establish exact duplicates/)).toBeVisible();
});
it('reports a failed cached comparison image without implying original damage', async () => {
  render(<ComparisonViewer assets={assets} status={status} onClose={vi.fn()} onError={vi.fn()} />);
  fireEvent.error(await screen.findByRole('img', { name: assets[0]!.name }));
  expect(screen.getByRole('status')).toHaveTextContent(
    'Cached preview failed to load. The original has not been changed.',
  );
});
