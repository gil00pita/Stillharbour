import { useCallback, useEffect, useState, type ReactNode } from 'react';
import type { CopyManifest, LibraryRoot } from '../bindings';
import { errorMessage, libraryData } from '../ipc';

export function RootPartitionCopyWorkflow({
  manifest,
  initialRootId = '',
  onActivityChange,
  children,
}: {
  manifest: CopyManifest;
  initialRootId?: string | undefined;
  onActivityChange?: ((active: boolean) => void) | undefined;
  children: (root: string, activity: (active: boolean) => void) => ReactNode;
}) {
  const ids = [...new Set(manifest.files.map((file) => file.rootId))];
  const [roots, setRoots] = useState<LibraryRoot[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [choice, setChoice] = useState(initialRootId);
  const [busy, setBusy] = useState(false);
  const activity = useCallback(
    (active: boolean) => {
      setBusy(active);
      onActivityChange?.(active);
    },
    [onActivityChange],
  );
  useEffect(() => {
    let active = true;
    libraryData({ kind: 'status' }, 'status')
      .then((status) => {
        if (active) {
          setRoots(status.roots);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [manifest.id, retry]);
  const root = roots?.find((root) => root.id === choice);
  const allowed =
    ids.includes(choice) && root?.mode === 'managed' && root.availability === 'online';
  return (
    <section aria-label="Copy source libraries">
      <h3>Copy one source library at a time</h3>
      <p className="help">
        This selection spans {ids.length} libraries. Each library has its own output folder, native
        review, approval and job. A completed job covers only its reviewed files.
      </p>
      {!roots && !error && <p role="status">Loading source libraries…</p>}
      {error && (
        <div role="alert">
          {error}{' '}
          <button onClick={() => setRetry((value) => value + 1)}>Retry source libraries</button>
        </div>
      )}
      {roots && (
        <label>
          Source library to copy
          <select
            aria-label="Source library to copy"
            value={choice}
            disabled={busy}
            onChange={(event) => setChoice(event.target.value)}
          >
            <option value="">Choose a source library</option>
            {ids.map((id) => {
              const source = roots.find((root) => root.id === id);
              const count = manifest.files.filter((file) => file.rootId === id).length;
              return (
                <option
                  key={id}
                  value={id}
                  disabled={
                    !source || source.mode !== 'managed' || source.availability !== 'online'
                  }
                >
                  {source?.label ?? 'Unavailable source library'} — {count} files
                  {source?.mode === 'referenced'
                    ? ' — read-only'
                    : source?.availability !== 'online'
                      ? ' — unavailable'
                      : ''}
                </option>
              );
            })}
          </select>
        </label>
      )}
      {choice && !allowed && roots && (
        <p role="alert">
          This source library is unavailable or has no managed grant. Review its location and
          permissions before copying.
        </p>
      )}
      {allowed && children(choice, activity)}
    </section>
  );
}
