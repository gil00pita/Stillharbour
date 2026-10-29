import { useEffect, useState } from 'react';
import type { LibraryChangeKind, LibraryChangePage, LibraryStatus } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
const labels: Record<LibraryChangeKind, string> = {
  added: 'Added',
  moved: 'Moved',
  updated: 'Updated',
  removed: 'Removed',
  reappeared: 'Reappeared',
};
export function LibraryChanges({ status }: { status: LibraryStatus }) {
  const [open, setOpen] = useState(false);
  const [before, setBefore] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const requestKey = JSON.stringify([before, status.revision, attempt]);
  const [state, setState] = useState<{
    page: LibraryChangePage | null;
    cursor: string | null;
    key: string;
    error: string | null;
  }>({ page: null, cursor: null, key: '', error: null });
  useEffect(() => {
    if (!open) return;
    let active = true;
    libraryData({ kind: 'changes', before, limit: 25 }, 'changes')
      .then((page) => {
        if (active) setState({ page, cursor: before, key: requestKey, error: null });
      })
      .catch((error) => {
        if (active)
          setState((previous) => ({ ...previous, key: requestKey, error: errorMessage(error) }));
      });
    return () => {
      active = false;
    };
  }, [open, before, requestKey]);
  const loading = open && state.key !== requestKey;
  const error = state.key === requestKey ? state.error : null;
  const page = state.cursor === before ? state.page : null;
  return (
    <>
      {status.monitoring?.map((monitor) => (
        <p className="help library-monitor" key={monitor.rootId}>
          <strong>
            {status.roots.find((root) => root.id === monitor.rootId)?.label ?? 'Library'}
          </strong>
          {' · '}
          {!['watching', 'periodic'].includes(monitor.mode) &&
            `${monitor.mode.replaceAll('_', ' ')} · `}
          {monitor.message}
        </p>
      ))}
      <details
        className="library-changes"
        onToggle={(event) => {
          setOpen(event.currentTarget.open);
          if (event.currentTarget.open && !open) setAttempt((value) => value + 1);
        }}
      >
        <summary>Library changes</summary>
        <p className="help">
          Observed changes after the first completed scan. Originals stay unchanged by Stillharbour.
        </p>
        {loading && <p role="status">Checking library changes…</p>}
        {error && (
          <p role="alert">
            {error} <button onClick={() => setAttempt(attempt + 1)}>Retry changes</button>
          </p>
        )}
        {page?.changes.length === 0 && !loading && !error && <p>No changes recorded yet.</p>}
        {page && (
          <ol className="library-change-list">
            {page.changes.map((change) => (
              <li key={change.sequence}>
                <div>
                  <strong>{labels[change.kind]}</strong>
                  {' · '}
                  <bdi>{change.rootLabel}</bdi>
                  {change.entryKind === 'folder' && ' · Folder'}
                </div>
                {change.previousPath && (
                  <p>
                    <bdi>{change.previousPath}</bdi>
                  </p>
                )}
                {change.path && change.path !== change.previousPath && (
                  <p>
                    {change.kind === 'moved' && <span className="help">To: </span>}
                    <bdi>{change.path}</bdi>
                  </p>
                )}
                <time className="help" dateTime={change.observedAt}>
                  {new Date(change.observedAt).toLocaleString()}
                </time>
              </li>
            ))}
          </ol>
        )}
        <div className="library-change-navigation">
          {before && (
            <button disabled={loading} onClick={() => setBefore(null)}>
              Latest changes
            </button>
          )}
          {page?.nextBefore && (
            <button disabled={loading} onClick={() => setBefore(page.nextBefore)}>
              Older changes
            </button>
          )}
        </div>
      </details>
    </>
  );
}
