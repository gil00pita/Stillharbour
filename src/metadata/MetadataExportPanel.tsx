import { Field } from '@chakra-ui/react';
import { ExportHistory } from './ExportHistory';
import { useId, useState } from 'react';
import type {
  AssetSelection,
  DirectorySelection,
  MetadataExportKind,
  MetadataExportReview,
} from '../bindings';
import { errorMessage, library, libraryData, pickDirectory } from '../ipc';
import { Button, Select } from '../components/ui/controls';

export function MetadataExportPanel({
  selection,
  selectionCount,
  onBusy,
}: {
  selection: AssetSelection | null;
  selectionCount: number;
  onBusy: (busy: boolean) => void;
}) {
  const kindId = useId();
  const [kind, setKind] = useState<MetadataExportKind>('catalog');
  const [directory, setDirectory] = useState<DirectorySelection | null>(null);
  const [review, setReview] = useState<MetadataExportReview | null>(null);
  const [accepted, setAccepted] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  async function preview() {
    setBusy(true);
    onBusy(true);
    setError(null);
    setNotice('');
    setReview(null);
    setAccepted(false);
    try {
      const chosen = await pickDirectory();
      if (!chosen) {
        setNotice('Export folder selection canceled. Nothing was written to the destination.');
        return;
      }
      setDirectory(chosen);
      setReview(
        await libraryData(
          {
            kind: 'metadata_export_preview',
            export_kind: kind,
            selection: kind === 'xmp' ? selection : null,
            expected_count: kind === 'xmp' ? selectionCount : 0,
            selection_id: chosen.id,
          },
          'metadata_export_review',
        ),
      );
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
      onBusy(false);
    }
  }
  async function save() {
    if (!review || !directory || !accepted) return;
    setBusy(true);
    onBusy(true);
    setError(null);
    setNotice('');
    try {
      const result = await libraryData(
        {
          kind: 'metadata_export_save',
          id: review.id,
          digest: review.digest,
          selection_id: directory.id,
        },
        'metadata_export_saved',
      );
      setNotice(`${result.files} new files saved to ${result.destination}. ${result.message}`);
      setReview(null);
      setAccepted(false);
    } catch (error) {
      setError(errorMessage(error));
      setReview(null);
      setAccepted(false);
    } finally {
      setBusy(false);
      onBusy(false);
    }
  }
  return (
    <section aria-labelledby="metadata-export-title">
      <h3 id="metadata-export-title">Export catalog or XMP copies</h3>
      <p className="help">
        Export into a separate folder. Original photos, sidecars and existing outputs are never
        replaced. Catalog copies contain organization and references, not your photo backup.
      </p>
      <Field.Root>
        <Field.Label htmlFor={kindId}>Export type</Field.Label>
        <Select
          id={kindId}
          value={kind}
          disabled={busy}
          onChange={(event) => {
            setKind(event.target.value as MetadataExportKind);
            setReview(null);
            setAccepted(false);
          }}
        >
          <option value="catalog">Portable SQLite catalog copy</option>
          <option value="xmp">XMP copies for selected media</option>
        </Select>
      </Field.Root>
      {kind === 'xmp' && (
        <p>
          {selectionCount.toLocaleString()} selected · up to 200 media and 64 MiB per review.
          Ambiguous source associations require resolution before export.
        </p>
      )}
      <Button
        disabled={busy || (kind === 'xmp' && (!selection || selectionCount > 200))}
        onClick={() => void preview()}
      >
        Choose export folder & preview
      </Button>
      {error && <p role="alert">{error}</p>}
      {review && (
        <div className="metadata-export-review">
          <h4>
            Review {review.files.length} new export {review.files.length === 1 ? 'file' : 'files'}
          </h4>
          <p>
            <bdi>{review.destination}</bdi>
          </p>
          <p>{review.message}</p>
          <details>
            <summary>Files and verified byte counts</summary>
            <ul>
              {review.files.slice(0, 80).map((file) => (
                <li key={file.name}>
                  <bdi>{file.name}</bdi> · {file.bytes} bytes
                </li>
              ))}
            </ul>
            {review.files.length > 80 && (
              <p>
                {review.files.length - 80} additional files are included in the association
                manifest.
              </p>
            )}
          </details>
          <label className="check-row">
            <input
              type="checkbox"
              checked={accepted}
              disabled={busy}
              onChange={(event) => setAccepted(event.target.checked)}
            />
            I reviewed these new outputs and their privacy/association warning.
          </label>
          <Button disabled={!accepted || busy} onClick={() => void save()}>
            Save reviewed export
          </Button>
        </div>
      )}
      {busy && (
        <Button
          onClick={() => {
            void library({ kind: 'metadata_export_cancel' }).catch((error) =>
              setError(errorMessage(error)),
            );
          }}
        >
          Cancel export
        </Button>
      )}
      <p role="status">{busy ? 'Preparing or verifying export…' : notice}</p>
      <ExportHistory />
    </section>
  );
}
