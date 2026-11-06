import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
import type {
  AnnotationPatch,
  AssetQuery,
  DateBucket,
  FolderEntry,
  LibraryStatus,
  SelectionToken,
} from '../bindings';
import { libraryData } from '../ipc';
import { defaultQuery, type Section } from '../library/model';
import { useAssetFeed } from '@/hooks/use-asset-feed';
import { selectionScope, toggleSelection } from '../library/selection';
type BrowseState = { query: AssetQuery; period: string | null; index: number | null };
type BrowseAction =
  | { type: 'filter'; patch: Partial<AssetQuery> }
  | { type: 'period'; period: string }
  | { type: 'index'; index: number };
function reducer(state: BrowseState, action: BrowseAction): BrowseState {
  if (action.type === 'filter')
    return { query: { ...state.query, ...action.patch }, period: null, index: null };
  if (action.type === 'index') return { ...state, period: null, index: action.index };
  return { ...state, period: action.period, index: null };
}
export function useLibraryController({
  initialQuery,
  initialSelectedId,
  defaults,
  status,
  section,
  onError,
  onRefresh,
  onWindowChange,
}: {
  initialQuery?: AssetQuery | undefined;
  initialSelectedId?: string | null;
  defaults?: import('../bindings').AppPreferences;
  status: LibraryStatus;
  section: Section;
  onError: (error: unknown) => void;
  onRefresh: () => Promise<void>;
  onWindowChange?: (followTop?: boolean) => void;
}) {
  const [browse, dispatch] = useReducer(reducer, {
    query: {
      ...defaultQuery,
      rootId: defaults?.defaultRootId ?? null,
      sort: defaults?.sort ?? defaultQuery.sort,
      ...initialQuery,
    },
    period: null,
    index: null,
  });
  const [ids, setIds] = useState<ReadonlySet<string>>(
    () => new Set(initialSelectedId ? [initialSelectedId] : []),
  );
  const [token, setToken] = useState<SelectionToken | null>(null);
  const anchor = useRef<string | null>(null);
  const epoch = useRef(0);
  const [buckets, setBuckets] = useState<DateBucket[]>([]);
  const [folders, setFolders] = useState<FolderEntry[]>([]);
  const [tags, setTags] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const query = useMemo<AssetQuery>(
    () => ({
      ...browse.query,
      deferPendingPreviews: !(
        browse.query.mapBounds ||
        browse.query.mapCell ||
        browse.query.location
      ),
    }),
    [browse.query],
  );
  const feed = useAssetFeed(
    query,
    browse.period,
    status.revision,
    onError,
    onWindowChange,
    browse.index,
  );
  const { page, loading } = feed;
  useEffect(() => {
    let active = true;
    libraryData({ kind: 'date_buckets', query }, 'buckets')
      .then((value) => {
        if (active) setBuckets(value);
      })
      .catch(onError);
    return () => {
      active = false;
    };
  }, [query, status.revision, onError]);
  useEffect(() => {
    let active = true;
    if (section === 'Folders' && query.rootId)
      libraryData({ kind: 'folders', root_id: query.rootId }, 'folders')
        .then((value) => {
          if (active) setFolders(value);
        })
        .catch(onError);
    if (section === 'Tags')
      libraryData({ kind: 'tags' }, 'tags')
        .then((value) => {
          if (active) setTags(value);
        })
        .catch(onError);
    return () => {
      active = false;
    };
  }, [section, query.rootId, status.revision, onError]);
  useEffect(() => {
    epoch.current++;
  }, [section]);
  function clear() {
    if (busy) return;
    setIds(new Set());
    setToken(null);
    anchor.current = null;
  }
  function filter(patch: Partial<AssetQuery>) {
    if (busy) return;
    const filters = { ...patch };
    delete filters.deferPendingPreviews;
    epoch.current++;
    dispatch({ type: 'filter', patch: filters });
    clear();
  }
  function select(id: string, range: boolean) {
    if (busy || token || loading) return;
    const next = toggleSelection(
      ids,
      page?.assets.map((asset) => asset.id) ?? [],
      anchor.current,
      id,
      range,
    );
    if (next.size > 500) {
      setNotice('Choose up to 500 individual photos, or use Select all results in Filter.');
      return;
    }
    setIds(next);
    if (!range || !anchor.current) anchor.current = id;
  }
  function selectGroup(group: string[]) {
    if (busy || token || loading) return;
    const next = new Set(ids);
    const remove = group.every((id) => next.has(id));
    group.forEach((id) => {
      if (remove) next.delete(id);
      else next.add(id);
    });
    if (next.size > 500)
      setNotice('Choose up to 500 individual photos, or use Select all results in Filter.');
    else setIds(next);
  }
  async function freeze() {
    const generation = epoch.current;
    try {
      const value = await libraryData({ kind: 'freeze_selection', query }, 'selection');
      if (generation === epoch.current) {
        setToken(value.count ? value : null);
        setIds(new Set());
        setNotice(
          value.count
            ? `${value.count.toLocaleString()} matching photos selected.`
            : 'No matching photos remain.',
        );
        return true;
      }
      return false;
    } catch (error) {
      onError(error);
      return false;
    }
  }
  async function annotate(patch: AnnotationPatch, id?: string) {
    setBusy(true);
    try {
      const count = await libraryData(
        { kind: 'annotate', selection: selectionScope(ids, token, id), patch },
        'updated',
      );
      setNotice(`Saved catalog notes for ${count.toLocaleString()} photos.`);
      await onRefresh();
    } catch (error) {
      onError(error);
      throw error;
    } finally {
      setBusy(false);
    }
  }
  async function addTags(tags: string[]) {
    setBusy(true);
    try {
      const count = await libraryData(
        { kind: 'add_tags', selection: selectionScope(ids, token), tags },
        'updated',
      );
      setNotice(`Added catalog tags to ${count.toLocaleString()} photos.`);
      await onRefresh();
      return true;
    } catch (error) {
      onError(error);
      return false;
    } finally {
      setBusy(false);
    }
  }
  async function compare() {
    if (token || ids.size < 2 || ids.size > 4) return null;
    const generation = epoch.current;
    setBusy(true);
    try {
      const data = await Promise.all(
        [...ids].map((id) => libraryData({ kind: 'detail', asset_id: id }, 'detail')),
      );
      return generation === epoch.current ? data.map((detail) => detail.asset) : null;
    } catch (error) {
      onError(error);
      return null;
    } finally {
      setBusy(false);
    }
  }
  return {
    onError,
    browse,
    query,
    page,
    loading,
    queryError: feed.error && !page ? feed.error.message : null,
    feed,
    buckets,
    folders,
    tags,
    notice,
    selection: {
      ids,
      count: token?.count ?? ids.size,
      frozen: Boolean(token),
      busy,
      scope: token
        ? { kind: 'token' as const, id: token.id }
        : ids.size
          ? { kind: 'ids' as const, ids: [...ids] }
          : null,
    },
    filter,
    select,
    selectGroup,
    clear,
    freeze,
    annotate,
    addTags,
    compare,
    retry: feed.retry,
    jump: (period: string) => {
      dispatch({ type: 'period', period });
    },
    jumpIndex: (index: number) =>
      dispatch({ type: 'index', index: Math.max(0, Math.round(index)) }),
    previous: feed.previous,
    next: feed.next,
  };
}

export type LibraryController = ReturnType<typeof useLibraryController>;
