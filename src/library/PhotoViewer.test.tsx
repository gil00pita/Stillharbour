import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { LibraryRoot, LibraryStatus } from '../bindings';
import { libraryData } from '../ipc';
import { PhotoViewer } from './PhotoViewer';
import { defaultFilter, defaultQuery } from './model';
import { fixtureAsset } from './__tests__/fixtures';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: () => 'thumbnail://cache/' }));
const a = fixtureAsset('a');
const b = fixtureAsset('b');
const c = fixtureAsset('c');
const root: LibraryRoot = {
  id: 'generated-root',
  label: 'Generated library',
  displayPath: 'Generated fixture',
  mode: 'referenced',
  isBase: true,
  availability: 'online',
  filter: defaultFilter,
  lastScan: null,
  fileCount: 3,
};
const status: LibraryStatus = {
  roots: [root],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
afterEach(() => vi.unstubAllGlobals());
beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
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
      if (request.kind === 'neighbors')
        return request.asset_id === 'a'
          ? { previous: null, next: b }
          : request.asset_id === 'b'
            ? { previous: a, next: c }
            : { previous: b, next: null };
      if (request.kind === 'detail')
        return { asset: request.asset_id === 'a' ? a : b, root, companions: [], rawMetadata: null };
      throw new Error('Unexpected request');
    });
});
function props() {
  return {
    initial: a,
    pageAssets: [a, b],
    query: { ...defaultQuery, name: 'Generated' },
    status,
    onClose: vi.fn(),
    onAnnotate: vi.fn(),
    onError: vi.fn(),
  };
}
it('opens an immersive read-only preview and navigates beyond the initial gallery page in the same filtered query', async () => {
  const callbacks = props();
  render(<PhotoViewer {...callbacks} />);
  expect(screen.getByRole('dialog', { name: 'Photo viewer' })).toBeVisible();
  expect(screen.queryByText(/not a full-resolution original/)).not.toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole('button', { name: 'Next photo' })).toBeEnabled());
  fireEvent.click(screen.getByRole('button', { name: 'Next photo' }));
  expect(screen.getByRole('heading', { name: b.name })).toBeVisible();
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'neighbors',
        asset_id: 'b',
        query: expect.objectContaining({ name: 'Generated' }),
      }),
      'neighbors',
    ),
  );
  await waitFor(() => expect(screen.getByRole('button', { name: 'Next photo' })).toBeEnabled());
  fireEvent.click(screen.getByRole('button', { name: 'Next photo' }));
  expect(screen.getByRole('heading', { name: c.name })).toBeVisible();
});
it('provides zoom, optional filmstrip, information panel and an explicit return to gallery', async () => {
  const callbacks = props();
  render(<PhotoViewer {...callbacks} />);
  expect(screen.queryByRole('navigation', { name: 'Nearby photos' })).not.toBeInTheDocument();
  expect(screen.queryByRole('complementary', { name: 'Photo details' })).not.toBeInTheDocument();
  const controls = screen.getByRole('group', { name: 'Photo controls' });
  for (const name of ['Zoom out', 'Zoom in', 'Fit image', 'Favorite photo', 'Info', 'Filmstrip']) {
    const button = screen.getByRole('button', { name });
    expect(controls).toContainElement(button);
    expect(button).toHaveAttribute('title');
    expect(button.textContent).toBe('');
  }
  await waitFor(() => expect(screen.getByRole('slider', { name: 'Preview zoom' })).toBeEnabled());
  fireEvent.change(screen.getByRole('slider', { name: 'Preview zoom' }), {
    target: { value: '200' },
  });
  expect(screen.getByText('200% of fit')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Fit image' }));
  expect(screen.getByText('100% of fit')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Filmstrip' }));
  expect(screen.getByRole('navigation', { name: 'Nearby photos' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Info' }));
  expect(await screen.findByRole('heading', { name: 'Photo details' })).toHaveFocus();
  fireEvent.click(screen.getByRole('button', { name: 'Close photo viewer' }));
  expect(callbacks.onClose).toHaveBeenCalledOnce();
});
it('labels offline cached previews without equating them with an accessible original', async () => {
  render(
    <PhotoViewer
      {...props()}
      status={{ ...status, roots: [{ ...root, availability: 'offline' }] }}
    />,
  );
  const original = document.querySelector('.original-image')!;
  fireEvent.error(original);
  fireEvent.error(original);
  expect(screen.getByText(/Original unavailable/)).toBeVisible();
  await screen.findByRole('img', { name: a.name });
});
it('shows failed previews and retry while keeping navigation and original protection explicit', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'thumbnails')
      return request.keys.map((key) => ({
        ...key,
        state: 'unavailable',
        error: 'Unsupported image format',
      }));
    return { previous: null, next: b };
  });
  const callbacks = props();
  render(<PhotoViewer {...callbacks} />);
  expect(await screen.findByText('Unsupported image format')).toBeVisible();
  expect(screen.getByText('The original has not been changed.')).toBeVisible();
  const before = vi.mocked(libraryData).mock.calls.length;
  fireEvent.click(screen.getByRole('button', { name: 'Retry preview' }));
  await waitFor(() => expect(vi.mocked(libraryData).mock.calls.length).toBeGreaterThan(before));
  fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Delete' });
  expect(callbacks.onAnnotate).not.toHaveBeenCalled();
});
it('shows navigation failure without pretending the collection ended', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'neighbors') throw new Error('Catalog temporarily unavailable');
    return [];
  });
  render(<PhotoViewer {...props()} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog temporarily unavailable');
  expect(screen.getByRole('button', { name: 'Retry navigation' })).toBeVisible();
});

it('enables viewer controls for a cached gallery tier when all requested tiers are unavailable offline', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) =>
    request.kind === 'thumbnails'
      ? request.keys.map((key) => ({
          ...key,
          state: 'unavailable',
          ...(key.edge === 2048 ? { fallbackEdge: 1024 } : {}),
          error: 'Reconnect the original',
        }))
      : { previous: null, next: b },
  );
  render(
    <PhotoViewer
      {...props()}
      status={{ ...status, roots: [{ ...root, availability: 'offline' }] }}
    />,
  );
  expect(await screen.findByRole('img', { name: a.name })).toHaveAttribute(
    'src',
    expect.stringContaining('/1024/'),
  );
  expect(screen.getByRole('slider', { name: 'Preview zoom' })).toBeEnabled();
});

it('keeps the page viewer nonmodal, does not dismiss on image click, and closes on Escape', async () => {
  const callbacks = props();
  render(<PhotoViewer {...callbacks} />);
  const viewer = screen.getByRole('dialog', { name: 'Photo viewer' });
  expect(viewer).toHaveAttribute('aria-modal', 'false');
  expect(screen.getByRole('button', { name: 'Close photo viewer' })).toHaveFocus();
  const image = await screen.findByRole('img', { name: a.name });
  fireEvent.click(image);
  expect(callbacks.onClose).not.toHaveBeenCalled();
  fireEvent.keyDown(viewer, { key: 'Escape' });
  expect(callbacks.onClose).toHaveBeenCalledOnce();
});

it('places X on the right and the catalog-backed animated heart before Info in the photo controls', async () => {
  let favorite = false;
  const callbacks = props();
  callbacks.onAnnotate.mockImplementation(async (patch) => {
    favorite = patch.favorite;
  });
  const original = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) =>
    request.kind === 'detail'
      ? { asset: { ...a, favorite }, root, companions: [], rawMetadata: null }
      : original(request, kind),
  );
  const { container } = render(<PhotoViewer {...callbacks} />);
  expect(container.querySelector('.viewer-toolbar')?.lastElementChild).toHaveAccessibleName(
    'Close photo viewer',
  );
  const heart = screen.getByRole('button', { name: 'Favorite photo' });
  const controls = screen.getByRole('group', { name: 'Photo controls' });
  expect(controls).toContainElement(heart);
  expect(heart.nextElementSibling).toBe(screen.getByRole('button', { name: 'Info' }));
  expect(container.querySelector('.viewer-photo')).toContainElement(controls);
  expect(heart).toHaveAttribute('aria-pressed', 'false');
  fireEvent.click(heart);
  await waitFor(() => expect(heart).toHaveAttribute('aria-pressed', 'true'));
  expect(callbacks.onAnnotate).toHaveBeenCalledWith(
    { favorite: true, rating: null, tags: null },
    'a',
  );
  expect(heart).toHaveClass('is-animating');
  fireEvent.animationEnd(heart.querySelector('span')!);
  fireEvent.click(heart);
  await waitFor(() => expect(heart).toHaveAttribute('aria-pressed', 'false'));
});
it('keeps a failed favorite write unfilled and reports the failure', async () => {
  const callbacks = props();
  callbacks.onAnnotate.mockRejectedValue(new Error('Catalog unavailable'));
  render(<PhotoViewer {...callbacks} />);
  const heart = screen.getByRole('button', { name: 'Favorite photo' });
  fireEvent.click(heart);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  expect(heart).toHaveAttribute('aria-pressed', 'false');
  expect(heart).not.toHaveClass('is-animating');
});
