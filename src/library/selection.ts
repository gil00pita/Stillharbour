import type { AssetSelection, SelectionToken } from '../bindings';

export function selectionScope(
  ids: ReadonlySet<string>,
  token: SelectionToken | null,
  id?: string,
): AssetSelection {
  if (id) return { kind: 'ids', ids: [id] };
  if (token) return { kind: 'token', id: token.id };
  return { kind: 'ids', ids: [...ids] };
}
export function toggleSelection(
  ids: ReadonlySet<string>,
  ordered: string[],
  anchor: string | null,
  id: string,
  range: boolean,
): Set<string> {
  const next = new Set(ids);
  const start = ordered.indexOf(anchor ?? '');
  const end = ordered.indexOf(id);
  if (range && start >= 0 && end >= 0)
    ordered.slice(Math.min(start, end), Math.max(start, end) + 1).forEach((id) => next.add(id));
  else if (next.has(id)) next.delete(id);
  else next.add(id);
  return next;
}
