import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { MaintenancePreview, MaintenanceResult, MaintenanceTarget } from '../bindings';
import { errorMessage, libraryData } from '../ipc';

export function LibraryMaintenance({
  target,
  onClose,
  onComplete,
}: {
  target: MaintenanceTarget;
  onClose: () => void;
  onComplete: (result: MaintenanceResult) => Promise<void>;
}) {
  const [preview, setPreview] = useState<MaintenancePreview | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [completed, setCompleted] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    let active = true;
    heading.current?.focus();
    libraryData({ kind: 'prepare_maintenance', target }, 'maintenance_preview')
      .then((value) => {
        if (active) {
          setPreview(value);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      })
      .finally(() => {
        if (active) {
          setBusy(false);
        }
      });
    return () => {
      active = false;
    };
  }, [target, attempt]);
  async function confirm(event: FormEvent) {
    event.preventDefault();
    if (!preview || busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        { kind: 'apply_maintenance', plan_id: preview.id },
        'maintenance_done',
      );
      setCompleted(true);
      setPreview(null);
      await onComplete(result);
    } catch (error) {
      setError(errorMessage(error));
      setPreview(null);
    } finally {
      setBusy(false);
    }
  }
  const removing = target.kind === 'library';
  return (
    <section className="maintenance-review" aria-labelledby="maintenance-review-title">
      <form
        aria-label="Review library maintenance"
        onSubmit={(event) => void confirm(event)}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !busy) {
            event.preventDefault();
            onClose();
          }
        }}
      >
        <h3 id="maintenance-review-title" tabIndex={-1} ref={heading}>
          {completed ? 'Completed' : removing ? 'Remove this library?' : 'Clear preview cache?'}
        </h3>
        {completed ? (
          <p role="status">
            {removing
              ? 'Library removed. Original photos and sidecars are unchanged.'
              : 'Indexed preview cache cleared. Libraries and catalog notes are unchanged.'}
          </p>
        ) : (
          <>
            <p>
              {removing
                ? 'This forgets the selected folder, its indexed records, catalog favorites, ratings, tags and scan history, and clears its indexed cached previews. You can add the folder again, but its catalog notes will not return. Library change history, including previous paths, remains in this catalog.'
                : 'Clear indexed cached previews for every library. Originals, library settings and catalog notes stay unchanged. Previews regenerate when accessible photos are viewed; offline photos need their originals reconnected.'}
            </p>
            <p className="help">
              Original photos and sidecars stay unchanged. Only indexed previews are cleared;
              unexpected files are retained.
            </p>
            {preview?.rootDisplayPath && (
              <p className="path">
                <bdi>{preview.rootDisplayPath}</bdi>
              </p>
            )}
            {preview && (
              <p className="help">
                <strong>
                  <bdi>{preview.rootLabel ?? 'All libraries'}</bdi>
                </strong>{' '}
                · {removing ? `${preview.assetCount.toLocaleString()} catalog records · ` : ''}
                {preview.cachedPreviews.toLocaleString()} cached previews ·{' '}
                {BigInt(preview.cacheBytes).toLocaleString()} bytes
              </p>
            )}
          </>
        )}
        {error && (
          <p role="alert" className="error">
            {completed
              ? 'The action completed, but refreshing Settings failed. Reopen Settings to refresh.'
              : error}
          </p>
        )}
        <div className="settings-library-actions">
          <button type="button" disabled={busy} onClick={onClose}>
            {completed ? 'Done' : 'Cancel'}
          </button>
          {!completed && !preview && !busy && (
            <button
              type="button"
              onClick={() => {
                setBusy(true);
                setError(null);
                setAttempt((value) => value + 1);
              }}
            >
              Review again
            </button>
          )}
          {!completed && (
            <button
              type="submit"
              className={removing ? 'destructive' : ''}
              disabled={busy || !preview}
            >
              {busy
                ? 'Please wait…'
                : removing
                  ? 'Remove library and its previews'
                  : 'Clear all cached previews'}
            </button>
          )}
        </div>
      </form>
    </section>
  );
}
