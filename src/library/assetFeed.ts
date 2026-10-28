import type { AssetPage, AssetQuery, LibraryRequest } from '../bindings';
import { libraryData } from '../ipc';
export const FEED_PAGE_SIZE = 240;
export const FEED_WINDOW_PAGES = 3;
export type FeedDirection = 'replace' | 'refresh' | 'before' | 'after';
export function firstRequest(query: AssetQuery, period: string | null): LibraryRequest {
  return period
    ? { kind: 'seek_date', query, period, limit: FEED_PAGE_SIZE }
    : { kind: 'query', query, cursor: null, limit: FEED_PAGE_SIZE };
}
async function readChainPages(
  query: AssetQuery,
  request: LibraryRequest,
  count: number,
): Promise<AssetPage[]> {
  const page = await libraryData(request, 'page');
  if (count <= 1 || !page.nextCursor || !page.assets.length) return [page];
  return [
    page,
    ...(await readChain(
      query,
      { kind: 'query', query, cursor: page.nextCursor, limit: FEED_PAGE_SIZE },
      count - 1,
    )),
  ];
}
export function mergeFeed(pages: AssetPage[]): AssetPage | null {
  const last = pages.at(-1);
  if (!last) return null;
  const seen = new Set<string>();
  return {
    ...last,
    assets: pages
      .flatMap((page) => page.assets)
      .filter((asset) => {
        if (seen.has(asset.id)) return false;
        seen.add(asset.id);
        return true;
      }),
  };
}

export function readChain(
  query: AssetQuery,
  request: LibraryRequest,
  count: number,
): Promise<AssetPage[]> {
  return readChainPages(
    query,
    request,
    Number.isFinite(count) ? Math.max(1, Math.min(FEED_WINDOW_PAGES, Math.floor(count))) : 1,
  );
}
