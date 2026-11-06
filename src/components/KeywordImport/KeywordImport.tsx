import { useState } from 'react';
import { Button } from '@/components/Button';
import { errorMessage, libraryData } from '../../ipc';
import { metadata } from '../../metadata/ipc';
export function KeywordImport({
  assetId,
  onImported,
}: {
  assetId: string;
  onImported: () => void;
}) {
  const [restore, setRestore] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  async function apply() {
    setBusy(true);
    setError(null);
    setNotice('');
    try {
      const status = await libraryData({ kind: 'status' }, 'status');
      await metadata(
        {
          kind: 'import_keywords',
          asset_id: assetId,
          expected_revision: status.revision,
          restore_removed: restore,
        },
        'notes',
      );
      setNotice(
        'Keyword import reviewed and saved in the catalog. Removed labels stay suppressed unless explicitly restored.',
      );
      onImported();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <details>
      <summary>Import observed keywords</summary>
      <p className="help">
        Import the retained source observations into manual tags. Labels you previously removed stay
        suppressed unless you explicitly restore them. No embedded metadata or sidecars are
        rewritten.
      </p>
      <label className="check-row">
        <input
          type="checkbox"
          checked={restore}
          disabled={busy}
          onChange={(event) => setRestore(event.target.checked)}
        />
        Restore previously removed observed labels too
      </label>
      <Button disabled={busy} onClick={() => void apply()}>
        Import keywords into catalog
      </Button>
      {error && <p role="alert">{error}</p>}
      <p role="status">{notice}</p>
    </details>
  );
}
