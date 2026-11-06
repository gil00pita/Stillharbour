import { useState } from 'react';
import { Button } from '@/components/Button';
import type { MetadataExportRecord } from '../../bindings';
import { errorMessage } from '../../ipc';
import { metadata } from '../../metadata/ipc';
export function ExportHistory() {
  const [entries, setEntries] = useState<MetadataExportRecord[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  async function load() {
    setBusy(true);
    setError(null);
    try {
      setEntries(await metadata({ kind: 'export_history' }, 'export_history'));
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <details>
      <summary>Export completion and retained partials</summary>
      <p className="help">
        Only a completed receipt proves that the reviewed export finished. Interrupted outputs and
        stages are retained. Preview a new copy to retry; old approvals are never replayed after
        restart.
      </p>
      <Button disabled={busy} onClick={() => void load()}>
        Load export history
      </Button>
      {error && <p role="alert">{error}</p>}
      {entries && (
        <ul>
          {entries.map((entry) => (
            <li key={entry.id}>
              <bdi>{entry.id}</bdi> ·{' '}
              {entry.lastPhase === 'completed' ? 'Completed' : 'Incomplete — retained for review'} ·{' '}
              {entry.filesVerified} files verified · {entry.updatedAt}
            </li>
          ))}
        </ul>
      )}
      {entries?.length === 0 && <p>No export receipts yet.</p>}
    </details>
  );
}
