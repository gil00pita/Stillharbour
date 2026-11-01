import { useState, type FormEvent } from 'react';
import type { ImportDestinationPreference, LibraryRoot } from '../bindings';
import { errorMessage } from '../ipc';

export function ImportDestinationSettings({
  catalogId,
  preference,
  roots,
  onSave,
}: {
  catalogId: string | null;
  preference: ImportDestinationPreference | null;
  roots: LibraryRoot[];
  onSave: ((rootId: string | null) => Promise<ImportDestinationPreference>) | null;
}) {
  const current = preference?.catalogId === catalogId ? (preference.rootId ?? '') : '';
  const [choice, setChoice] = useState(current);
  const [dirty, setDirty] = useState(false);
  const [pending, setPending] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [observed, setObserved] = useState(current);
  if (observed !== current) {
    setObserved(current);
    if (!dirty) setChoice(current);
  }
  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!catalogId || !onSave || pending) return;
    setPending(true);
    setError(null);
    setSaved(false);
    try {
      const result = await onSave(choice || null);
      if (result.catalogId !== catalogId || result.rootId !== (choice || null))
        throw new Error(
          'The saved destination belongs to another catalog or choice. Refresh and review again.',
        );
      setDirty(false);
      setSaved(true);
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <section className="settings-section" aria-labelledby="import-destination-title">
      <h2 id="import-destination-title">Import destination</h2>
      <p className="help">
        A preference for this catalog, separate from its base library and browsing defaults. It does
        not grant writes, choose a folder or approve copying. Unavailable and selected source
        destinations remain blocked during planning.
      </p>
      <form aria-label="Import destination preference" onSubmit={(event) => void save(event)}>
        <label>
          Preferred import destination
          <select
            aria-label="Preferred import destination"
            value={choice}
            disabled={!catalogId || !onSave || pending}
            onChange={(event) => {
              setChoice(event.target.value);
              setDirty(true);
              setSaved(false);
            }}
          >
            <option value="">Choose for each plan</option>
            {choice && !roots.some((root) => root.id === choice) && (
              <option value={choice} disabled>
                Unavailable saved destination
              </option>
            )}
            {roots.map((root) => (
              <option key={root.id} value={root.id} disabled={root.mode !== 'managed'}>
                {root.label}
                {root.mode !== 'managed'
                  ? ' — read-only'
                  : root.availability !== 'online'
                    ? ' — unavailable'
                    : ''}
              </option>
            ))}
          </select>
        </label>
        <button type="submit" disabled={!catalogId || !onSave || pending || !dirty}>
          {pending ? 'Saving import destination…' : 'Save import destination'}
        </button>
        {saved && (
          <p role="status">
            Import destination saved for this catalog. Folder grants and existing plans are
            unchanged.
          </p>
        )}
        {error && <p role="alert">{error}</p>}
      </form>
    </section>
  );
}
