import { Field } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useCallback, useEffect, useState, type ReactNode } from 'react';
import type { CopyManifest, LibraryRoot } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

export function RootPartitionCopyWorkflow({
  manifest,
  initialRootId = '',
  onActivityChange,
  children,
  purpose = 'copy',
}: {
  manifest: CopyManifest;
  initialRootId?: string | undefined;
  onActivityChange?: ((active: boolean) => void) | undefined;
  children: (root: string, activity: (active: boolean) => void) => ReactNode;
  purpose?: 'copy' | 'import';
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
  const readable = (root: LibraryRoot | undefined) =>
    root?.mode === 'managed' || (purpose === 'import' && root?.mode === 'referenced');
  const allowed = ids.includes(choice) && readable(root) && root?.availability === 'online';
  return (
    <section aria-label={purpose === 'copy' ? 'Copy source libraries' : 'Import source libraries'}>
      <h3>
        {purpose === 'copy'
          ? 'Copy one source library at a time'
          : 'Compare one source library at a time'}
      </h3>
      <p className="help">
        This selection spans {ids.length} libraries.{' '}
        {purpose === 'copy'
          ? 'Each library has its own output folder, native review, approval and job. A completed job covers only its reviewed files.'
          : 'Choose one source for each comparison and import plan. Close its report before switching sources. Copies and skips need separate approval.'}
      </p>
      {!roots && !error && <p role="status">Loading source libraries…</p>}
      {error && (
        <div role="alert">
          {error}{' '}
          <Button onClick={() => setRetry((value) => value + 1)}>Retry source libraries</Button>
        </div>
      )}
      {roots && (
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>
                {purpose === 'copy' ? 'Source library to copy' : 'Source library to compare'}
              </span>
            </Field.Label>
            <NativeSelect
              aria-label={
                purpose === 'copy' ? 'Source library to copy' : 'Source library to compare'
              }
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
                    disabled={!readable(source) || source?.availability !== 'online'}
                  >
                    {source?.label ?? 'Unavailable source library'} — {count} files
                    {source?.mode === 'referenced'
                      ? purpose === 'copy'
                        ? ' — read-only'
                        : ' — read-only source'
                      : source?.availability !== 'online'
                        ? ' — unavailable'
                        : ''}
                  </option>
                );
              })}
            </NativeSelect>
          </label>
        </Field.Root>
      )}
      {choice && !allowed && roots && (
        <p role="alert">
          {purpose === 'copy'
            ? 'This source library is unavailable or has no managed grant. Review its location and permissions before copying.'
            : 'This source library is unavailable. Review its location before comparing.'}
        </p>
      )}
      {allowed && children(choice, activity)}
    </section>
  );
}
