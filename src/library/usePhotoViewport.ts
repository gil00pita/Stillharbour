import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { AssetSummary } from '../bindings';
import { PHOTO_GUTTER, photoRows, type DateGrouping, type ViewMode } from './model';

export function usePhotoViewport({
  assets,
  mode,
  target,
  grouping,
  onVisibleDate,
  pageKey,
}: {
  assets: AssetSummary[];
  mode: ViewMode;
  target: number;
  grouping: DateGrouping;
  onVisibleDate: (date: string, scrolling: boolean) => void;
  pageKey: string;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const anchor = useRef<string | null>(null);
  const visibleAnchor = useRef<string | null>(null);
  const visibleOffset = useRef(0);
  const anchorOffset = useRef(0);
  const remember = useRef<() => void>(() => {});
  const previousAssets = useRef(assets);
  const previousPage = useRef(pageKey);
  const pageReset = useRef(false);
  const restoredScroll = useRef<number | null>(null);
  const widthRef = useRef(700);
  const [width, setWidth] = useState(700);
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    let pendingWidth = widthRef.current;
    let frame = 0;
    const observer = new ResizeObserver((entries) => {
      pendingWidth = Math.max(160, entries[0]?.contentRect.width ?? 700);
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        if (Math.abs(pendingWidth - widthRef.current) < 0.5) return;
        remember.current();
        widthRef.current = pendingWidth;
        setWidth(pendingWidth);
      });
    });
    observer.observe(element);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);
  const rows = useMemo(
    () => photoRows(assets, width, mode, target, grouping),
    [assets, width, mode, target, grouping],
  );
  // React Compiler is disabled; the mutable virtualizer stays at the I/O boundary.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => viewport.current,
    estimateSize: (index) => (rows[index]?.height ?? target) + PHOTO_GUTTER,
    overscan: 2,
    isScrollingResetDelay: 1000,
    getItemKey: (index) => rows[index]?.assets[0]?.id ?? index,
    onChange: (instance) => {
      const first = instance
        .getVirtualItems()
        .find((item) => item.end > (instance.scrollOffset ?? 0));
      const asset = first ? rows[first.index]?.assets[0] : assets[0];
      if (!pageReset.current) {
        visibleAnchor.current = asset?.id ?? null;
        visibleOffset.current = Math.max(0, (instance.scrollOffset ?? 0) - (first?.start ?? 0));
      }
      onVisibleDate(
        (grouping === 'added' ? asset?.addedAt : asset?.date) ?? '',
        instance.isScrolling,
      );
    },
  });
  const virtual = virtualizer.getVirtualItems();
  const rememberAnchor = useCallback(() => {
    const first = virtualizer
      .getVirtualItems()
      .find((row) => row.end > (viewport.current?.scrollTop ?? 0));
    anchor.current = first ? (rows[first.index]?.assets[0]?.id ?? null) : null;
    anchorOffset.current = Math.max(0, (viewport.current?.scrollTop ?? 0) - (first?.start ?? 0));
  }, [rows, virtualizer]);
  useLayoutEffect(() => {
    remember.current = rememberAnchor;
  }, [rememberAnchor]);
  const scrollToOffset = useCallback(
    (offset: number) => {
      virtualizer.scrollToOffset(offset);
      restoredScroll.current = viewport.current?.scrollTop ?? null;
    },
    [virtualizer],
  );
  useEffect(() => {
    if (previousPage.current === pageKey) return;
    previousPage.current = pageKey;
    pageReset.current = true;
    anchor.current = null;
    visibleAnchor.current = null;
    scrollToOffset(0);
  }, [pageKey, scrollToOffset]);
  useEffect(() => {
    const preservedId = anchor.current ?? visibleAnchor.current;
    const preservedOffset = anchor.current ? anchorOffset.current : visibleOffset.current;
    virtualizer.measure();
    const changed = previousAssets.current !== assets;
    if (pageReset.current && changed) {
      pageReset.current = false;
      anchor.current = null;
      visibleAnchor.current = null;
      scrollToOffset(0);
    } else if (!pageReset.current && (anchor.current || changed)) {
      const id = preservedId;
      const index = rows.findIndex((row) => row.assets.some((asset) => asset.id === id));
      if (index >= 0) {
        const offset = preservedOffset;
        const start = rows.slice(0, index).reduce((sum, row) => sum + row.height + PHOTO_GUTTER, 0);
        scrollToOffset(start + Math.min(offset, Math.max(0, (rows[index]?.height ?? 1) - 1)));
      }
    }
    previousAssets.current = assets;
    anchor.current = null;
  }, [assets, mode, rows, virtualizer, scrollToOffset]);
  return {
    viewport,
    rows,
    virtual,
    height: virtualizer.getTotalSize(),
    remember: rememberAnchor,
    isRestorationScroll: () => {
      const expected = restoredScroll.current;
      restoredScroll.current = null;
      return expected !== null && Math.abs((viewport.current?.scrollTop ?? 0) - expected) < 1;
    },
    scrollToAsset: (id: string) => {
      const index = rows.findIndex((row) => row.assets.some((asset) => asset.id === id));
      if (index >= 0) virtualizer.scrollToIndex(index, { align: 'auto' });
      restoredScroll.current = viewport.current?.scrollTop ?? null;
    },
  };
}
