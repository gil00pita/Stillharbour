import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { AssetPage, AssetQuery, LibraryRequest } from '../bindings';
import { errorMessage } from '../ipc';
import {
  FEED_PAGE_SIZE,
  FEED_WINDOW_PAGES,
  firstRequest,
  mergeFeed,
  readChain,
  type FeedDirection,
} from '../library/assetFeed';
type FeedState = {
  key: string;
  queryKey: string;
  navigationId: number;
  revision: string;
  pages: AssetPage[];
  hasBefore: boolean;
  error: { direction: FeedDirection; message: string } | null;
};
export function useAssetFeed(
  query: AssetQuery,
  period: string | null,
  revision: string,
  onError: (error: unknown) => void,
  onWindowChange?: (followTop?: boolean) => void,
  index: number | null = null,
) {
  const queryKey = JSON.stringify(query);
  const key = JSON.stringify([query, period, index]);
  const [state, setState] = useState<FeedState>({
    key: '',
    queryKey: '',
    navigationId: 0,
    revision: '',
    pages: [],
    hasBefore: false,
    error: null,
  });
  const [busy, setBusy] = useState(false);
  const gate = useRef(false);
  const replacements = useRef<{ running: boolean; pending: (() => Promise<void>) | null }>({
    running: false,
    pending: null,
  });
  const generation = useRef(0);
  const activeKey = useRef('');
  const request = useCallback(
    async (direction: FeedDirection) => {
      if (gate.current || activeKey.current !== key) return;
      const current = state.key === key ? state : { ...state, pages: [], hasBefore: false };
      const first = current.pages[0]?.assets[0];
      const tail = current.pages.at(-1)?.nextCursor;
      if (direction === 'before' && (!current.hasBefore || !first)) return;
      if (direction === 'after' && !tail) return;
      let native: LibraryRequest = firstRequest(query, period, index);
      if (direction === 'after')
        native = { kind: 'query', query, cursor: tail ?? null, limit: FEED_PAGE_SIZE };
      if ((direction === 'before' || (direction === 'refresh' && current.hasBefore)) && first)
        native = {
          kind: 'seek_asset',
          query,
          asset_id: first.id,
          position: direction === 'before' ? 'before' : 'at',
          limit: FEED_PAGE_SIZE,
        };
      const token = generation.current;
      gate.current = true;
      setBusy(true);
      try {
        let pages = await readChain(
          query,
          native,
          direction === 'refresh' ? Math.max(1, current.pages.length) : 1,
        );
        if (direction === 'refresh' && !pages[0]?.assets.length && (pages[0]?.total ?? 0) > 0) {
          pages = await readChain(
            query,
            { kind: 'query', query, cursor: null, limit: FEED_PAGE_SIZE },
            Math.max(1, current.pages.length),
          );
        }
        if (token !== generation.current) return;
        onWindowChange?.(direction === 'refresh' && !current.hasBefore && !period);
        const observedRevision = pages.at(-1)!.revision;
        setState((previous) => {
          if (direction === 'before' && !pages[0]?.assets.length)
            return { ...previous, revision: observedRevision, hasBefore: false, error: null };
          if (direction === 'after' && !pages[0]?.assets.length)
            return {
              ...previous,
              revision: observedRevision,
              pages: previous.pages.map((page, index) =>
                index === previous.pages.length - 1 ? { ...page, nextCursor: null } : page,
              ),
              error: null,
            };
          if (direction === 'replace' || direction === 'refresh')
            return {
              key,
              queryKey,
              navigationId: previous.navigationId,
              revision: observedRevision,
              pages,
              hasBefore:
                direction === 'refresh' ? current.hasBefore : Boolean(period) || index !== null,
              error: null,
            };
          const combined =
            direction === 'before' ? [...pages, ...previous.pages] : [...previous.pages, ...pages];
          const overflow = combined.length > FEED_WINDOW_PAGES;
          return {
            key,
            queryKey,
            navigationId: previous.navigationId,
            revision: observedRevision,
            pages:
              direction === 'before'
                ? combined.slice(0, FEED_WINDOW_PAGES)
                : combined.slice(-FEED_WINDOW_PAGES),
            hasBefore:
              direction === 'before'
                ? pages[0]!.assets.length === FEED_PAGE_SIZE
                : previous.hasBefore || overflow,
            error: null,
          };
        });
      } catch (error) {
        if (token === generation.current) {
          setState((previous) => ({
            ...previous,
            key,
            error: { direction, message: errorMessage(error) },
            revision,
          }));
          onError(error);
        }
      } finally {
        if (token === generation.current) {
          gate.current = false;
          setBusy(false);
        }
      }
    },
    [key, queryKey, query, period, index, revision, state, onError, onWindowChange],
  );
  useEffect(() => {
    activeKey.current = key;
    const token = ++generation.current;
    gate.current = true;
    setBusy(true);
    const replace = () =>
      readChain(query, firstRequest(query, period, index), 1)
        .then((pages) => {
          if (token === generation.current)
            setState({
              key,
              queryKey,
              navigationId: token,
              revision: pages.at(-1)!.revision,
              pages,
              hasBefore: Boolean(period) || index !== null,
              error: null,
            });
        })
        .catch((error) => {
          if (token === generation.current) {
            setState({
              key,
              queryKey,
              navigationId: token,
              revision: '0',
              pages: [],
              hasBefore: false,
              error: { direction: 'replace', message: errorMessage(error) },
            });
            onError(error);
          }
        })
        .finally(() => {
          if (token === generation.current) {
            gate.current = false;
            setBusy(false);
          }
        });
    const scheduler = replacements.current;
    scheduler.pending = replace;
    async function drain() {
      if (scheduler.running) return;
      scheduler.running = true;
      try {
        while (scheduler.pending) {
          const next = scheduler.pending;
          scheduler.pending = null;
          await next();
        }
      } finally {
        scheduler.running = false;
      }
    }
    void drain();
    return () => {
      generation.current = token + 1;
      gate.current = false;
      scheduler.pending = null;
    };
  }, [key, queryKey, query, period, index, onError]);
  useEffect(() => {
    if (
      !busy &&
      state.key === key &&
      state.pages.length &&
      BigInt(state.revision || '0') < BigInt(revision) &&
      !state.error
    )
      void request('refresh');
  }, [busy, state, key, revision, request]);
  const matched = state.key === key;
  const retain = matched || state.queryKey === queryKey;
  const page = useMemo(() => (retain ? mergeFeed(state.pages) : null), [retain, state.pages]);
  return {
    key: page ? `${state.key}:${state.navigationId}` : key,
    page,
    loading: !matched || (busy && !page),
    busy,
    error: matched ? state.error : null,
    hasBefore: retain && state.hasBefore,
    hasAfter: Boolean(page?.nextCursor),
    next: () => void request('after'),
    previous: () => void request('before'),
    retry: () => void request(state.error?.direction ?? 'replace'),
  };
}
