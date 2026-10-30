import { useEffect, useId, useRef, useState } from 'react';
import type { DirectorySelection, LibraryRoot, RootMode, ScanFilter } from '../bindings';
import { defaultFilter, normalizeFileExtensions } from './model';
import { Switch } from '../components/Switch';
import { ScanOptions } from './ScanOptions';
import { scanFilterFromForm } from './scanForm';

export function RootSetup({
  selected,
  root,
  busy,
  onSave,
  onCancel,
  onError,
  defaults,
  embedded = false,
  scanRunning = false,
}: {
  selected?: DirectorySelection | undefined;
  root?: LibraryRoot | undefined;
  busy: boolean;
  onSave: (label: string, mode: RootMode, filter: ScanFilter) => Promise<void>;
  onCancel: () => void;
  onError: (error: unknown) => void;
  defaults?: Pick<import('../bindings').AppPreferences, 'rootMode' | 'scanFilter'>;
  embedded?: boolean;
  scanRunning?: boolean;
}) {
  const [label, setLabel] = useState(root?.label ?? 'Photo library');
  const [mode, setMode] = useState<RootMode>(root?.mode ?? defaults?.rootMode ?? 'managed');
  const labelInput = useRef<HTMLInputElement>(null);
  const focusedSelection = useRef<string | undefined>(undefined);
  const selectionId = selected?.id;
  const identityAlertId = useId();
  const identityUnavailable = !root && selected?.identityAvailable === false;
  useEffect(() => {
    if (selectionId && !busy && focusedSelection.current !== selectionId) {
      labelInput.current?.focus();
      focusedSelection.current = selectionId;
    }
  }, [selectionId, busy]);
  const [filter, setFilter] = useState<ScanFilter>(() => ({
    ...(root?.filter ?? defaults?.scanFilter ?? defaultFilter),
    includeGlobs: [],
    extensions: normalizeFileExtensions(
      (root?.filter ?? defaults?.scanFilter ?? defaultFilter).extensions,
    ),
    fromDate: null,
    throughDate: null,
  }));
  return (
    <form
      className={`root-setup ${embedded ? 'inline-scan-settings' : 'selected-folder'}`}
      onSubmit={(event) => {
        event.preventDefault();
        if (!busy && !scanRunning && !identityUnavailable && event.currentTarget.reportValidity())
          void onSave(label, mode, scanFilterFromForm(filter, event.currentTarget));
      }}
    >
      <div className="folder-information">
        {!embedded && <h3>{root ? 'Scan settings' : 'Selected folder'}</h3>}
        <p className="path">
          <bdi>{selected?.displayPath ?? root?.displayPath}</bdi>
        </p>
      </div>
      {!root && <span className="selection-state">Not scanned</span>}
      <div className="form-grid">
        <label>
          Library name
          <input
            ref={labelInput}
            readOnly={Boolean(root)}
            disabled={busy}
            value={label}
            maxLength={128}
            required
            onChange={(event) => setLabel(event.target.value)}
          />
        </label>
        <div className="mode-field">
          <span className="field-label">
            Folder mode · {mode === 'managed' ? 'Managed' : 'Read only'}
          </span>
          <Switch
            label="Read only"
            checked={mode === 'referenced'}
            disabled={busy}
            onChange={(readOnly) => setMode(readOnly ? 'referenced' : 'managed')}
          />
        </div>
      </div>
      <p className="help">
        {mode === 'referenced'
          ? 'Originals stay in this folder. Scanning only reads them.'
          : 'Managed permission is saved for future approved operations. This version only reads originals.'}
      </p>
      {identityUnavailable && (
        <div className="error" role="alert" id={identityAlertId}>
          <strong>Scanning is unavailable for this folder</strong>
          <p>
            Stillharbour cannot verify folder identity on this platform or volume. Save &amp; Start
            scanning is disabled to protect your library. On Windows, select an ordinary local NTFS
            folder; network volumes, other filesystems, links and cloud placeholders are not
            supported.
          </p>
        </div>
      )}
      <ScanOptions filter={filter} disabled={busy} onChange={setFilter} onError={onError} />
      <div className="folder-actions">
        <button type="button" onClick={onCancel} disabled={busy}>
          {embedded ? 'Discard scan changes' : 'Cancel'}
        </button>
        <button
          className="primary"
          type="submit"
          disabled={busy || scanRunning || (!root && !selected?.identityAvailable)}
          aria-describedby={identityUnavailable ? identityAlertId : undefined}
        >
          {scanSubmitLabel(busy, scanRunning, Boolean(root))}
        </button>
      </div>
    </form>
  );
}

function scanSubmitLabel(busy: boolean, running: boolean, existing: boolean) {
  if (busy) return 'Starting scan…';
  if (running) return 'Scan running…';
  return existing ? 'Save & Rescan' : 'Save & Start scanning';
}
