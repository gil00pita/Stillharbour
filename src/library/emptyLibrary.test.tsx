import { render, screen } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { LibraryRoot, LibraryStatus } from '../bindings';
import { libraryData } from '../ipc';
import { LibraryScreen } from './LibraryScreen';
import { fixturePreferences } from './__tests__/fixtures';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));

const root: LibraryRoot = {
  id: 'generated-root',
  label: 'Generated library',
  displayPath: '/generated',
  mode: 'managed',
  isBase: true,
  availability: 'online',
  filter: fixturePreferences().scanFilter,
  lastScan: '2026-10-31',
  fileCount: 0,
};

beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'query')
        return { assets: [], total: 0, nextCursor: null, revision: '1' } as never;
      return [] as never;
    });
});

function libraryView(roots: LibraryRoot[], jobs: LibraryStatus['jobs'] = []) {
  return (
    <LibraryScreen
      defaults={fixturePreferences()}
      status={{
        roots,
        jobs,
        revision: '1',
        previewGeneration: '0',
        metadataAvailable: true,
        previewAvailable: true,
      }}
      section="Photos"
      onError={vi.fn()}
      onRefresh={vi.fn()}
      onSettings={vi.fn()}
      onRelink={vi.fn()}
    />
  );
}

function show(roots: LibraryRoot[], jobs: LibraryStatus['jobs'] = []) {
  return render(libraryView(roots, jobs));
}

it('shows the themed illustration only after an empty library has finished loading', async () => {
  show([root]);
  expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
  expect(document.querySelector('.photo-scanning-art')).not.toBeInTheDocument();
  expect(await screen.findByRole('heading', { name: 'Your library is empty' })).toBeVisible();
  expect(document.querySelector('.empty-library-art')).toHaveAttribute('aria-hidden', 'true');
});

it.each([
  ['no library', []],
  ['filtered results in a populated library', [{ ...root, fileCount: 2 }]],
  ['offline library', [{ ...root, availability: 'offline' }]],
  ['unscanned library', [{ ...root, lastScan: null }]],
  ['another populated library', [root, { ...root, id: 'other', fileCount: 1 }]],
] as const)('does not illustrate %s as an empty library', async (_, roots) => {
  show([...roots]);
  await screen.findByRole('heading', { name: 'No matching photos yet' });
  expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
  expect(document.querySelector('.photo-scanning-art')).not.toBeInTheDocument();
});

it('does not show the illustration while previews are preparing', async () => {
  show([{ ...root, preparingPreviews: true }]);
  await screen.findByRole('heading', { name: 'Preparing your photos' });
  expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
  expect(document.querySelector('.photo-scanning-art')).toHaveAttribute('data-active', 'false');
});

it('does not show the illustration after a query failure', async () => {
  vi.mocked(libraryData).mockRejectedValue(new Error('Catalog unavailable.'));
  show([root]);
  await screen.findByRole('heading', { name: 'Library couldn’t be loaded' });
  expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
  expect(document.querySelector('.photo-scanning-art')).not.toBeInTheDocument();
});

it.each(['queued', 'running', 'cancel_requested'])(
  'does not show the illustration while a scan is %s',
  async (state) => {
    show(
      [root],
      [
        {
          id: 'generated-job',
          rootId: root.id,
          kind: 'scan',
          state,
          discovered: 0,
          indexed: 0,
          skipped: 0,
          errors: 0,
          message: 'Scanning generated fixture',
          startedAt: '2026-10-31',
        },
      ],
    );
    await screen.findByRole('heading', { name: 'Scanning your photos' });
    expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
    expect(document.querySelector('.photo-scanning-art')).toHaveAttribute('aria-hidden', 'true');
    expect(document.querySelector('.photo-scanning-art')).toHaveAttribute(
      'data-active',
      String(state !== 'cancel_requested'),
    );
    expect(
      screen.queryByRole('heading', { name: 'No matching photos yet' }),
    ).not.toBeInTheDocument();
  },
);

it('animates preview preparation and returns to the empty view after completion', async () => {
  const view = show(
    [{ ...root, preparingPreviews: true }],
    [
      {
        id: 'generated-previews',
        rootId: root.id,
        kind: 'thumbnails',
        state: 'running',
        discovered: 0,
        indexed: 0,
        skipped: 0,
        errors: 0,
        message: 'Preparing generated previews',
        startedAt: '2026-10-31',
      },
    ],
  );
  await screen.findByRole('heading', { name: 'Preparing your photos' });
  expect(document.querySelector('.photo-scanning-art')).toHaveAttribute('data-active', 'true');
  view.rerender(libraryView([root]));
  await screen.findByRole('heading', { name: 'Your library is empty' });
  expect(document.querySelector('.photo-scanning-art')).not.toBeInTheDocument();
});
