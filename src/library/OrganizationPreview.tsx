import { useEffect, useRef, useState, type FormEvent } from 'react';
import type {
  OrganizationIssue,
  OrganizationPreview as Preview,
  OrganizationTemplate,
  LibraryRoot,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { OrganizationCompanions } from './OrganizationCompanions';

const defaultPreset = {
  name: 'Year / date / original name',
  pattern: '{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
};
const presets = [
  defaultPreset,
  {
    name: 'Year / month / date and sequence',
    pattern: '{capture:%Y}/{capture:%m}/{capture:%Y%m%d_%H%M%S}_{sequence:04}.{ext}',
  },
  {
    name: 'Event / date / original name',
    pattern: '{event}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
  },
];
const issueLabels: Record<OrganizationIssue, string> = {
  missing_capture_date: 'No embedded capture date',
  missing_value: 'A required token value is missing',
  invalid_path: 'Name cannot be used under the portable naming policy',
  source_unavailable: 'Source is not currently recorded as available',
  non_unicode_path: 'Native name cannot be represented by this template',
  collision: 'Proposed paths collide',
  possible_case_collision: 'Proposed paths may collide on a case-insensitive destination',
};

export function OrganizationPreview({
  assetIds,
  revision,
  roots,
  onClose,
}: {
  assetIds: string[];
  revision: string;
  roots: Pick<LibraryRoot, 'id' | 'label'>[];
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const resultsCaption = useRef<HTMLTableCaptionElement>(null);
  const requestId = useRef(0);
  const [template, setTemplate] = useState<OrganizationTemplate>({
    pattern: defaultPreset.pattern,
    event: '',
    missingDate: 'undated',
  });
  const [preview, setPreview] = useState<Preview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  useEffect(() => {
    if (preview) {
      resultsCaption.current?.focus({ preventScroll: true });
      resultsCaption.current?.scrollIntoView({ block: 'start' });
    }
  }, [preview]);
  useEffect(() => {
    const element = dialog.current;
    const opener = document.activeElement;
    if (element && !element.open) element.showModal();
    return () => {
      requestId.current += 1;
      element?.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  function update(patch: Partial<OrganizationTemplate>) {
    setTemplate((value) => ({ ...value, ...patch }));
    setPreview(null);
    setError(null);
  }
  async function build(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!event.currentTarget.reportValidity()) return;
    const id = ++requestId.current;
    setBusy(true);
    setError(null);
    setPreview(null);
    try {
      const result = await libraryData(
        { kind: 'organization_preview', asset_ids: assetIds, template },
        'organization_preview',
      );
      if (requestId.current === id) setPreview(result);
    } catch (error) {
      if (requestId.current === id) setError(errorMessage(error));
    } finally {
      if (requestId.current === id) setBusy(false);
    }
  }
  const stale = preview !== null && preview.catalogRevision !== revision;
  const rows = preview?.rows.filter((row) =>
    `${row.sourcePath} ${row.targetRelativePath ?? ''}`
      .toLocaleLowerCase()
      .includes(search.toLocaleLowerCase()),
  );
  return (
    <dialog
      className="organization-preview"
      ref={dialog}
      aria-labelledby="organization-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header className="organization-heading">
        <div>
          <h2 id="organization-title">Preview organization</h2>
          <p className="help">
            {assetIds.length} explicitly selected photos · Originals stay in place
          </p>
        </div>
        <button onClick={onClose} aria-label="Close organization preview">
          Close
        </button>
      </header>
      <div className="organization-body">
        <p>Preview folder and filename changes using indexed metadata. Originals stay in place.</p>
        <form onSubmit={(event) => void build(event)} aria-busy={busy}>
          <fieldset disabled={busy}>
            <label>
              Preset
              <select
                value={presets.find((preset) => preset.pattern === template.pattern)?.pattern ?? ''}
                onChange={(event) => {
                  if (event.target.value) update({ pattern: event.target.value });
                }}
              >
                <option value="" disabled>
                  Custom template
                </option>
                {presets.map((preset) => (
                  <option key={preset.name} value={preset.pattern}>
                    {preset.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Path template
              <input
                required
                maxLength={1024}
                value={template.pattern}
                onChange={(event) => update({ pattern: event.target.value })}
                spellCheck={false}
                aria-describedby="organization-token-help"
              />
            </label>
            <p className="help" id="organization-token-help">
              Tokens:{' '}
              {
                '{original_stem}, {original_folder}, {ext}, {event}, {capture:%Y-%m-%d}, {sequence:04}'
              }
              . Keep the original extension. Unsafe names are blocked rather than silently renamed.
            </p>
            <div className="organization-options">
              <label>
                Event name
                <input
                  maxLength={128}
                  value={template.event}
                  onChange={(event) => update({ event: event.target.value })}
                />
              </label>
              <label>
                When capture date is missing
                <select
                  value={template.missingDate}
                  onChange={(event) =>
                    update({ missingDate: event.target.value === 'block' ? 'block' : 'undated' })
                  }
                >
                  <option value="undated">Use Undated / original filename</option>
                  <option value="block">Block the affected photo</option>
                </select>
              </label>
            </div>
            <button type="submit" className="primary">
              {busy ? 'Building preview…' : 'Build preview'}
            </button>
          </fieldset>
        </form>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {preview && (
          <section aria-label="Proposed organization">
            {stale && (
              <p role="alert">
                The catalog changed. Build a fresh preview before reviewing these paths.
              </p>
            )}
            <p role="status">
              {preview.proposed} proposed · {preview.blocked} blocked ·{' '}
              {BigInt(preview.selectedBytes).toLocaleString()} selected file bytes
            </p>
            <p className="help">
              Relative paths only. Destination conflicts, native case and Unicode rules, companions,
              metadata fidelity and free space still need review. Sequence numbers follow library ID
              and native source path order.
            </p>
            <label>
              Find a path
              <input
                type="search"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </label>
            <div className="organization-table">
              <table>
                <caption ref={resultsCaption} tabIndex={-1}>
                  Selected photos: current path, proposed relative path and review issues
                </caption>
                <thead>
                  <tr>
                    <th scope="col">Current path</th>
                    <th scope="col">Proposed path</th>
                    <th scope="col">Review</th>
                  </tr>
                </thead>
                <tbody>
                  {rows?.map((row) => (
                    <tr key={row.assetId}>
                      <td>
                        <bdi>{row.sourcePath}</bdi>
                        <small title={`Library ID: ${row.rootId}`}>
                          {roots.find((root) => root.id === row.rootId)?.label ??
                            'Unlisted library'}
                        </small>
                      </td>
                      <td>
                        <bdi>{row.targetRelativePath ?? 'No valid target'}</bdi>
                      </td>
                      <td>
                        {row.issues.length
                          ? row.issues.map((issue) => <p key={issue}>{issueLabels[issue]}</p>)
                          : 'Destination checks pending'}
                        {row.usedUndated && <p>Undated fallback; no embedded capture date</p>}
                        {row.companionCandidates > 0 && (
                          <p>
                            {row.companionCandidates} indexed companion candidates; bundle review
                            required
                          </p>
                        )}
                        <OrganizationCompanions
                          key={`${preview.fingerprint}:${row.assetId}`}
                          assetId={row.assetId}
                          previewRevision={preview.catalogRevision}
                          revision={revision}
                        />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {rows?.length === 0 && <p>No paths match your search.</p>}
            </div>
          </section>
        )}
      </div>
      <footer className="organization-footer">
        <p className="help" id="organization-execution-help">
          Copying and in-place changes are unavailable until destination planning, verification and
          recovery checks pass.
        </p>
        <button disabled aria-describedby="organization-execution-help">
          Create organized copy
        </button>
      </footer>
    </dialog>
  );
}
