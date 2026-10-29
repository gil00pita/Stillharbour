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
  scrollRect: { height: 200, width: 700 },
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
  expect(callbacks.onVisibleDate).toHaveBeenLastCalledWith('2026-03-01', false, 0);
  engine.scrollOffset = 200;
  engine.isScrolling = true;
  act(() => events.onChange?.(engine));
  const visibleDate = photoRows(assets, 700, 'gallery')[1]!.assets[0]!.date;
  expect(visibleDate).not.toBe(assets[0]!.date);
  expect(callbacks.onVisibleDate).toHaveBeenLastCalledWith(visibleDate, true, expect.any(Number));
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

it('requests only visible tiny previews while scrolling and restores overscan upgrades on settle', async () => {
  const callbacks = props();
  engine.isScrolling = true;
  const { rerender } = render(<AssetBrowser {...callbacks} />);
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
  const moving = vi.mocked(libraryData).mock.calls.at(-1)?.[0];
  expect(moving?.kind).toBe('thumbnails');
  if (moving?.kind !== 'thumbnails') throw new Error('Expected preview request');
  expect(moving.keys.length).toBeGreaterThan(0);
  expect(moving.keys.every((key) => key.edge === 32)).toBe(true);
  const visibleIds = photoRows(assets, 700, 'gallery')[0]!.assets.map((asset) => asset.id);
  expect(moving.keys.map((key) => key.assetId)).toEqual(visibleIds);
  engine.isScrolling = false;
  rerender(<AssetBrowser {...callbacks} />);
  await waitFor(() => {
    const settled = vi.mocked(libraryData).mock.calls.at(-1)?.[0];
    expect(settled?.kind === 'thumbnails' && settled.keys.some((key) => key.edge > 32)).toBe(true);
    expect(settled?.kind === 'thumbnails' && settled.keys.length).toBeGreaterThan(
      moving.keys.length,
    );
  });
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

it.each(['capture', 'added', 'undated'] as const)(
  'maps %s scrolling and dragging within a dense period to retained rows',
  (kind) => {
    const handle = createRef<PhotoViewportHandle>();
    const callbacks = props();
    const date = kind === 'undated' ? '' : '2026-03-01';
    const period = kind === 'undated' ? 'Undated' : '2026-03';
    const grouping = kind === 'added' ? 'added' : 'capture';
    const dense = Array.from({ length: 30 }, (_, index) => ({
      ...fixtureAsset(`${index}`, kind === 'added' ? '1964-07-01' : date),
      addedAt: date,
    }));
    const { container } = render(
      <AssetBrowser
        {...callbacks}
        assets={dense}
        grouping={grouping}
        mode="list"
        viewportHandle={handle}
        timeline={<nav />}
      />,
    );
    const viewport = container.querySelector<HTMLElement>('.asset-viewport')!;
    Object.defineProperty(viewport, 'clientHeight', { value: 200 });
    expect(viewport).toHaveClass('with-timeline');
    const rows = photoRows(dense, 700, 'list', undefined, grouping);
    const maximum = rows.reduce((sum, row) => sum + row.height + 4, 0) - 200;
    engine.scrollOffset = maximum / 2;
    act(() => events.onChange?.(engine));
    expect(callbacks.onVisibleDate).toHaveBeenLastCalledWith(date, false, 0.5);
    expect(handle.current!.scrollToPeriod(period, 0.75)).toBe(true);
    expect(engine.scrollToOffset).toHaveBeenLastCalledWith(maximum * 0.75);
    expect(handle.current!.scrollToPeriod('1964-07', 0.5)).toBe(false);
  },
);

it('loads earlier rows on upward timeline wheel input even when already at the retained top', () => {
  const handle = createRef<PhotoViewportHandle>();
  const onNearStart = vi.fn();
  const { container } = render(
    <AssetBrowser {...props()} viewportHandle={handle} onNearStart={onNearStart} />,
  );
  const viewport = container.querySelector<HTMLElement>('.asset-viewport')!;
  viewport.scrollBy = vi.fn();
  Object.defineProperty(viewport, 'clientHeight', { value: 500 });
  handle.current!.scrollBy(-120, 0);
  expect(onNearStart).toHaveBeenCalledOnce();
});

it('updates rendition requests when display density changes without altering row CSS width', async () => {
  const previous = window.devicePixelRatio;
  Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 1 });
  const { unmount } = render(<AssetBrowser {...props()} />);
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
  Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: 2 });
  fireEvent(window, new Event('resize'));
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(
          ([request]) =>
            request.kind === 'thumbnails' && request.keys.some((key) => key.edge === 1024),
        ),
    ).toBe(true),
  );
  unmount();
  Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: previous });
});

it('requests visible rows before earlier overscan rows under preview admission bounds', async () => {
  const callbacks = props();
  const { rerender } = render(<AssetBrowser {...callbacks} />);
  await waitFor(() => expect(libraryData).toHaveBeenCalled());
  vi.mocked(libraryData).mockClear();
  engine.scrollOffset = 200;
  rerender(<AssetBrowser {...callbacks} />);
  const expected = photoRows(assets, 700, 'gallery')[1]!.assets[0]!.id;
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.some(
          ([request]) => request.kind === 'thumbnails' && request.keys[0]?.assetId === expected,
        ),
    ).toBe(true),
  );
});

it('keeps the beginning visible when live updates insert newer photos', async () => {
  const handle = createRef<PhotoViewportHandle>();
  const original = props();
  const { rerender } = render(<AssetBrowser {...original} viewportHandle={handle} />);
  await waitFor(() => expect(handle.current).not.toBeNull());
  engine.scrollToOffset.mockClear();
  act(() => handle.current?.remember(true));
  rerender(
    <AssetBrowser
      {...original}
      assets={[fixtureAsset('new', '2026-10-29'), ...original.assets]}
      viewportHandle={handle}
    />,
  );
  expect(engine.scrollToOffset).toHaveBeenLastCalledWith(0);
});
