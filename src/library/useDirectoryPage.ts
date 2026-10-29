import { useEffect, useState } from 'react';
import type { DirectoryPage } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
export type FolderLocation = {
  rootId: string | null;
  directoryId: string;
  cursor: string | null;
  search: string;
};
export function useDirectoryPage(location: FolderLocation, revision: string) {
  const [attempt, setAttempt] = useState(0);
  const scope = JSON.stringify(location);
  const { rootId, directoryId, cursor, search } = location;
  const requestKey = JSON.stringify([scope, revision, attempt]);
  const [state, setState] = useState<{
    scope: string;
    key: string;
    page: DirectoryPage | null;
    error: string | null;
  }>({ scope: '', key: '', page: null, error: null });
  useEffect(() => {
    if (!rootId) return;
    let active = true;
    libraryData(
      {
        kind: 'browse_directory',
        root_id: rootId,
        directory_id: directoryId,
        name: search,
        cursor: cursor,
        limit: 240,
      },
      'directory',
    )
      .then(async (page) => {
        // A removed last entry can leave a valid cursor beyond the new end.
        // Recover with one bounded first-page request, not a false empty folder.
        if (active && cursor && page.total > 0 && !page.entries.length)
          page = await libraryData(
            {
              kind: 'browse_directory',
              root_id: rootId,
              directory_id: directoryId,
              name: search,
              cursor: null,
              limit: 240,
            },
            'directory',
          );
        if (active) setState({ scope, key: requestKey, page, error: null });
      })
      .catch((error) => {
        if (active)
          setState((previous) => ({
            scope,
            key: requestKey,
            page: previous.scope === scope ? previous.page : null,
            error: errorMessage(error),
          }));
      });
    return () => {
      active = false;
    };
  }, [rootId, directoryId, cursor, search, scope, requestKey]);
  return {
    page: location.rootId && state.scope === scope ? state.page : null,
    loading: Boolean(location.rootId && state.key !== requestKey),
    error: state.key === requestKey ? state.error : null,
    retry: () => setAttempt((value) => value + 1),
  };
}
