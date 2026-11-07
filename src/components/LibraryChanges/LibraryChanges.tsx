import { Box, List, Text } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useState } from 'react';
import type { LibraryChangeKind, LibraryChangePage, LibraryStatus } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
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
        <Text className="help library-monitor" key={monitor.rootId}>
          <strong>
            {status.roots.find((root) => root.id === monitor.rootId)?.label ?? 'Library'}
          </strong>
          {' · '}
          {!['watching', 'periodic'].includes(monitor.mode) &&
            `${monitor.mode.replaceAll('_', ' ')} · `}
          {monitor.message}
        </Text>
      ))}
      <details
        className="library-changes"
        onToggle={(event) => {
          setOpen(event.currentTarget.open);
          if (event.currentTarget.open && !open) setAttempt((value) => value + 1);
        }}
      >
        <summary>Library changes</summary>
        <Text className="help">
          Observed changes after the first completed scan. Originals stay unchanged by Stillharbour.
        </Text>
        {loading && <Text role="status">Checking library changes…</Text>}
        {error && (
          <Text role="alert">
            {error} <Button onClick={() => setAttempt(attempt + 1)}>Retry changes</Button>
          </Text>
        )}
        {page?.changes.length === 0 && !loading && !error && <Text>No changes recorded yet.</Text>}
        {page && (
          <List.Root as="ol" unstyled className="library-change-list">
            {page.changes.map((change) => (
              <List.Item unstyled key={change.sequence}>
                <Box>
                  <strong>{labels[change.kind]}</strong>
                  {' · '}
                  <bdi>{change.rootLabel}</bdi>
                  {change.entryKind === 'folder' && ' · Folder'}
                </Box>
                {change.previousPath && (
                  <Text>
                    <bdi>{change.previousPath}</bdi>
                  </Text>
                )}
                {change.path && change.path !== change.previousPath && (
                  <Text>
                    {change.kind === 'moved' && (
                      <Text as="span" className="help">
                        To:{' '}
                      </Text>
                    )}
                    <bdi>{change.path}</bdi>
                  </Text>
                )}
                <time className="help" dateTime={change.observedAt}>
                  {new Date(change.observedAt).toLocaleString()}
                </time>
              </List.Item>
            ))}
          </List.Root>
        )}
        <Box className="library-change-navigation">
          {before && (
            <Button disabled={loading} onClick={() => setBefore(null)}>
              Latest changes
            </Button>
          )}
          {page?.nextBefore && (
            <Button disabled={loading} onClick={() => setBefore(page.nextBefore)}>
              Older changes
            </Button>
          )}
        </Box>
      </details>
    </>
  );
}
