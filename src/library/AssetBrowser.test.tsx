import { createRef } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { AssetBrowser, type PhotoViewportHandle } from './AssetBrowser';
import { libraryData } from '../ipc';
import { fixtureAsset } from './__tests__/fixtures';
import { photoRows } from './model';
type ScrollEvent = {
  scrollOffset: number | null;
  isScrolling: boolean;
  getVirtualItems: () => { index: number; end: number }[];
};
const events = vi.hoisted(() => {
  const handlers: { onChange?: (instance: ScrollEvent) => void } = {};
  return handlers;
});

const engine = vi.hoisted(() => ({
  scrollOffset: 0,
  isScrolling: false,
  measure: vi.fn(),
  scrollToOffset: vi.fn(),
  scrollToIndex: vi.fn(),
  getTotalSize: () => 400,
  getVirtualItems: () => [
    { index: 0, start: 0, end: 200, key: 'row0' },
    { index: 1, start: 200, end: 400, key: 'row1' },
  ],
}));
vi.mock('@tanstack/react-virtual', () => ({
  useVirtualizer: (options: { onChange: (instance: ScrollEvent) => void }) => {
    events.onChange = options.onChange;
    return engine;
  },
}));
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  engine.scrollOffset = 0;
  engine.isScrolling = false;
  vi.mocked(libraryData).mockReset().mockResolvedValue([]);
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
});
const assets = Array.from({ length: 6 }, (_, index) =>
  fixtureAsset(`${index}`, index < 2 ? '2026-03-01' : '1964-07-01'),
);
function props() {
  return {
    assets,
    mode: 'gallery' as const,
    selected: new Set<string>(),
    selectionFrozen: false,
    activeId: null,
    onSelect: vi.fn(),
    onInspect: vi.fn(),
    onMode: vi.fn(),
    onError: vi.fn(),
    onVisibleDate: vi.fn(),
    pageKey: 'first-page',
  };
}
it('renders quiet Timeline captions with separate hover selection and photo controls', async () => {
  const callbacks = props();
  const { container } = render(<AssetBrowser {...callbacks} />);
  expect(container.querySelector('.asset-check')).toBeInTheDocument();
  expect(container.querySelector('.asset-name')).not.toBeInTheDocument();
  const photo = screen.getByRole('button', { name: 'Open Generated 0.jpg' });
  fireEvent.click(photo);
  expect(callbacks.onInspect).toHaveBeenCalledWith('0', photo);
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
});
it('reports the first actually visible row rather than an overscan row', async () => {
  const callbacks = props();
  render(<AssetBrowser {...callbacks} />);
  act(() => events.onChange?.(engine));
  expect(callbacks.onVisibleDate).toHaveBeenLastCalledWith('2026-03-01', false);
  engine.scrollOffset = 200;
  engine.isScrolling = true;
  act(() => events.onChange?.(engine));
  const visibleDate = photoRows(assets, 700, 'gallery')[1]!.assets[0]!.date;
  expect(visibleDate).not.toBe(assets[0]!.date);
  expect(callbacks.onVisibleDate).toHaveBeenLastCalledWith(visibleDate, true);
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
});
it('retains selection controls in Grid and makes no nonempty thumbnail batch in Text list', async () => {
  const callbacks = props();
  const { rerender } = render(
    <AssetBrowser {...callbacks} mode="grid" selected={new Set(['0'])} />,
  );
  expect(screen.getByRole('checkbox', { name: 'Select Generated 0.jpg' })).toBeChecked();
  rerender(<AssetBrowser {...callbacks} mode="list" selected={new Set(['0'])} />);
  expect(screen.getByRole('checkbox', { name: 'Select Generated 0.jpg' })).toBeChecked();
  await waitFor(() =>
    expect(libraryData).toHaveBeenLastCalledWith(
      expect.objectContaining({ kind: 'thumbnails', keys: [] }),
      'thumbnails',
    ),
  );
});

it('toggles a photo instead of opening it when selection is active, and supports Shift ranges', async () => {
  const callbacks = props();
  render(<AssetBrowser {...callbacks} selected={new Set(['0'])} />);
  fireEvent.click(screen.getByRole('button', { name: 'Open Generated 1.jpg' }), { shiftKey: true });
  expect(callbacks.onSelect).toHaveBeenCalledWith('1', true);
  expect(callbacks.onInspect).not.toHaveBeenCalled();
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
});
it('pauses gallery thumbnail requests while an immersive viewer owns the foreground', async () => {
  render(<AssetBrowser {...props()} previewPaused />);
  await Promise.resolve();
  expect(libraryData).not.toHaveBeenCalled();
});

it('coalesces resize bursts into one layout and retains the visible anchor', () => {
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible');
  let resize: ResizeObserverCallback = () => {};
  let frame: FrameRequestCallback = () => {};
  const request = vi.fn((callback: FrameRequestCallback) => {
    frame = callback;
    return 1;
  });
  vi.stubGlobal('requestAnimationFrame', request);
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: ResizeObserverCallback) {
        resize = callback;
      }
      observe() {}
      disconnect() {}
    },
  );
  const { container, unmount } = render(<AssetBrowser {...props()} />);
  engine.measure.mockClear();
  engine.scrollToOffset.mockClear();
  const element = container.querySelector<HTMLDivElement>('.asset-viewport')!;
  element.scrollTop = 230; // The second virtual row is visible, 30px below its start.
  const entry = (width: number): ResizeObserverEntry => ({
    target: element,
    contentRect: new DOMRectReadOnly(0, 0, width, 500),
    borderBoxSize: [],
    contentBoxSize: [],
    devicePixelContentBoxSize: [],
  });
  const observer: ResizeObserver = { observe() {}, unobserve() {}, disconnect() {} };
  act(() => {
    resize([entry(800)], observer);
    resize([entry(900)], observer);
  });
  expect(request).toHaveBeenCalledTimes(1);
  expect(engine.measure).not.toHaveBeenCalled();
  act(() => frame(0));
  expect(engine.measure).toHaveBeenCalledTimes(1);
  // The anchored photo moved into the first balanced row; retain its 30px offset.
  expect(engine.scrollToOffset).toHaveBeenCalledWith(30);
  expect(container.querySelector<HTMLElement>('.asset')!.style.width).toBe(
    `${photoRows(assets, 900, 'gallery')[0]!.widths[0]}px`,
  );
  unmount();
  vi.unstubAllGlobals();
});

it('forwards timeline wheel deltas to the photo viewport in pixels, lines and pages', () => {
  const handle = createRef<PhotoViewportHandle>();
  const { container } = render(<AssetBrowser {...props()} viewportHandle={handle} />);
  const viewport = container.querySelector<HTMLElement>('.asset-viewport')!;
  const scroll = vi.fn();
  viewport.scrollBy = scroll;
  Object.defineProperty(viewport, 'clientHeight', { value: 500 });
  handle.current!.scrollBy(120, 0);
  handle.current!.scrollBy(2, 1);
  handle.current!.scrollBy(2, 2);
  expect(scroll.mock.calls).toEqual([
    [{ top: 120, behavior: 'instant' }],
    [{ top: 32, behavior: 'instant' }],
    [{ top: 1000, behavior: 'instant' }],
  ]);
});

it('updates a hidden WebView resize even when animation frames are suspended', () => {
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  let resize: ResizeObserverCallback = () => {};
  const request = vi.fn();
  vi.stubGlobal('requestAnimationFrame', request);
  vi.stubGlobal('cancelAnimationFrame', vi.fn());
  vi.stubGlobal(
    'ResizeObserver',
    class {
      constructor(callback: ResizeObserverCallback) {
        resize = callback;
      }
      observe() {}
      disconnect() {}
    },
  );
  const { container } = render(<AssetBrowser {...props()} />);
  const element = container.querySelector<HTMLElement>('.asset-viewport')!;
  const entry: ResizeObserverEntry = {
    target: element,
    contentRect: new DOMRectReadOnly(0, 0, 363, 450),
    borderBoxSize: [],
    contentBoxSize: [],
    devicePixelContentBoxSize: [],
  };
  act(() => resize([entry], { observe() {}, unobserve() {}, disconnect() {} }));
  expect(request).not.toHaveBeenCalled();
  expect(container.querySelector<HTMLElement>('.asset')!.style.width).toBe(
    `${photoRows(assets, 363, 'gallery')[0]!.widths[0]}px`,
  );
  vi.unstubAllGlobals();
});
