import { useEffect, useRef, useState } from 'react';
import type {
  BundleConfirmation,
  BundleEntryKind,
  BundleInventory,
  BundleRelation,
  OrganizationRow,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';

const kinds: Record<BundleEntryKind, string> = {
  file: 'Regular file',
  directory: 'Folder',
  link_or_special: 'Link or special entry',
  placeholder: 'Cloud placeholder',
  unreadable: 'Metadata unavailable',
};
const relations: Record<BundleRelation, string> = {
  primary: 'Selected photo',
  same_stem: 'Possible same-stem companion',
  filename_sidecar: 'Possible filename sidecar',
  apple_double: 'Possible AppleDouble companion',
  additional: 'Other observed entry',
};
const PAGE_SIZE = 64;

export function BundleInventoryReview({
  planId,
  row,
  catalogRevision,
  stale,
  onClose,
  onChoose,
}: {
  planId: string;
  row: OrganizationRow;
  catalogRevision: string;
  stale: boolean;
  onClose: () => void;
  onChoose?: (decision: BundleConfirmation) => void;
}) {
  const [inventory, setInventory] = useState<BundleInventory | null>(null);
  const [selected, setSelected] = useState<Set<string>>(() => new Set());
  const [reviewed, setReviewed] = useState(false);
  const [page, setPage] = useState(0);
  const [confirmation, setConfirmation] = useState<BundleConfirmation | null>(null);
  const [history, setHistory] = useState<BundleConfirmation[] | null>(null);
  const [pending, setPending] = useState<'inventory' | 'confirm' | 'history' | null>(null);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  const heading = useRef<HTMLHeadingElement>(null);
  const caption = useRef<HTMLTableCaptionElement>(null);
  useEffect(() => {
    heading.current?.focus();
  }, []);
  useEffect(() => {
    if (inventory) caption.current?.focus();
  }, [inventory, page]);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  const busy = pending !== null;
  const changed =
    stale ||
    (inventory !== null &&
      (inventory.assetId !== row.assetId ||
        inventory.catalogRevision !== catalogRevision ||
        inventory.sourceRevision !== row.sourceRevision));
  const mandatoryMissing =
    inventory?.entries.some(
      (entry) =>
        !['primary', 'additional'].includes(entry.relation) &&
        (!entry.selectable || !selected.has(entry.id)),
    ) ?? true;
  const canConfirm =
    inventory?.inventoryComplete &&
    !changed &&
    reviewed &&
    inventory.entries.some((entry) => entry.relation === 'primary' && entry.kind === 'file') &&
    !mandatoryMissing &&
    selected.size <= 64;

  async function run(action: 'inventory' | 'confirm' | 'history') {
    if (busy || (action !== 'history' && changed)) return;
    const token = ++request.current;
    setPending(action);
    setError(null);
    try {
      if (action === 'inventory') {
        setInventory(null);
        setConfirmation(null);
        setSelected(new Set());
        setReviewed(false);
        setPage(0);
        const value = await libraryData(
          {
            kind: 'organization_bundle_inventory',
            asset_id: row.assetId,
            source_revision: row.sourceRevision,
          },
          'organization_bundle_inventory',
        );
        if (request.current !== token) return;
        setInventory(value);
        setSelected(
          new Set(
            value.entries
              .filter((entry) => entry.selectable && entry.relation !== 'additional')
              .map((entry) => entry.id),
          ),
        );
      } else if (action === 'history') {
        const value = await libraryData(
          { kind: 'organization_bundle_reviews', plan_id: planId, asset_id: row.assetId },
          'organization_bundle_reviews',
        );
        if (request.current === token) setHistory(value);
      } else if (inventory && canConfirm) {
        const value = await libraryData(
          {
            kind: 'organization_bundle_confirm',
            input: {
              planId,
              assetId: row.assetId,
              sourceRevision: row.sourceRevision,
              inventoryFingerprint: inventory.fingerprint,
              companionIds: [...selected].sort(),
              reviewedAllEntries: reviewed,
            },
          },
          'organization_bundle_confirmation',
        );
        if (request.current === token) {
          setConfirmation(value);
          onChoose?.(value);
          setHistory((previous) => [
            value,
            ...(previous ?? []).filter((record) => record.id !== value.id),
          ]);
        }
      }
    } catch (error) {
      if (request.current === token) setError(errorMessage(error));
    } finally {
      if (request.current === token) setPending(null);
    }
  }
  return (
    <section
      className="organization-destination bundle-inventory"
      aria-label="Complete companion inventory"
      aria-busy={busy}
    >
      <h3 ref={heading} tabIndex={-1}>
        Companions for <bdi>{row.sourcePath}</bdi>
      </h3>
      <p>
        Read all filenames in this photo’s immediate folder, including files excluded from the
        gallery. No file contents are read or changed. Folders, links and placeholders stay at
        source. This review creates no copy approval.
      </p>
      <button type="button" disabled={busy || changed} onClick={() => void run('inventory')}>
        {pending === 'inventory'
          ? 'Reading filenames…'
          : inventory
            ? 'Read filenames again'
            : 'Read all filenames'}
      </button>
      <button type="button" disabled={busy} onClick={() => void run('history')}>
        Load retained companion reviews
      </button>
      <button type="button" onClick={onClose}>
        Close companion inventory
      </button>
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {changed && (
        <p role="alert">
          The retained plan or source changed. Build a fresh saved plan before confirming
          companions.
        </p>
      )}
      {inventory && (
        <>
          <p>
            {inventory.entries.length} observed entries ·{' '}
            {selected.size +
              (inventory.entries.some((entry) => entry.relation === 'primary') ? 1 : 0)}{' '}
            files selected for review
          </p>
          {!inventory.inventoryComplete && (
            <p role="alert">
              The inventory is incomplete. Truncated, unreadable or unobserved entries block
              confirmation.
            </p>
          )}
          {(inventory.truncated || inventory.indexedLimitExceeded) && (
            <p>Inventory limits were reached. This is not the complete folder scope.</p>
          )}
          {inventory.unobservedCandidates.length > 0 && (
            <>
              <p>Indexed candidates not observed in this folder:</p>
              <ul>
                {inventory.unobservedCandidates.map((name) => (
                  <li key={name}>
                    <bdi>{name}</bdi>
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="organization-table">
            <table>
              <caption ref={caption} tabIndex={-1}>
                All observed folder entries · page {page + 1} of{' '}
                {Math.max(1, Math.ceil(inventory.entries.length / PAGE_SIZE))}
              </caption>
              <thead>
                <tr>
                  <th scope="col">Include</th>
                  <th scope="col">Name</th>
                  <th scope="col">Observation</th>
                </tr>
              </thead>
              <tbody>
                {inventory.entries.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE).map((entry) => (
                  <tr key={entry.id}>
                    <td>
                      <input
                        type="checkbox"
                        aria-label={`Include ${entry.name}`}
                        checked={entry.relation === 'primary' || selected.has(entry.id)}
                        disabled={
                          busy ||
                          !entry.selectable ||
                          (selected.size >= 64 && !selected.has(entry.id))
                        }
                        onChange={(event) => {
                          const checked = event.target.checked;
                          setSelected((previous) => {
                            const next = new Set(previous);
                            if (checked) next.add(entry.id);
                            else next.delete(entry.id);
                            return next;
                          });
                          setConfirmation(null);
                          setReviewed(false);
                        }}
                      />
                    </td>
                    <td>
                      <bdi>{entry.name}</bdi>
                      {!entry.nameRepresentable && (
                        <p>Name cannot be represented by the template.</p>
                      )}
                    </td>
                    <td>
                      {kinds[entry.kind]} · {relations[entry.relation]} ·{' '}
                      {entry.bytes === null
                        ? 'Size unavailable'
                        : `${BigInt(entry.bytes).toLocaleString()} bytes`}
                      <br />
                      {entry.catalogAssetId
                        ? `Catalog: ${entry.catalogPresence ?? 'unknown'}`
                        : 'Not indexed in the catalog'}
                      {entry.error && <p>{entry.error}</p>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button
            type="button"
            disabled={busy || page === 0}
            onClick={() => setPage((value) => value - 1)}
          >
            Previous filenames
          </button>
          <button
            type="button"
            disabled={busy || (page + 1) * PAGE_SIZE >= inventory.entries.length}
            onClick={() => setPage((value) => value + 1)}
          >
            Next filenames
          </button>
          <p>
            Include every possible same-stem, filename-sidecar and AppleDouble companion.
            Unsupported or ambiguous companions block confirmation. You may also include other
            regular files explicitly. At most 64 companions can be retained per photo.
          </p>
          {mandatoryMissing && (
            <p>
              Some possible companions are excluded or cannot be included. This bundle cannot be
              confirmed.
            </p>
          )}
          <label className="bundle-review-ack">
            <input
              type="checkbox"
              checked={reviewed}
              disabled={busy || !inventory.inventoryComplete || changed}
              onChange={(event) => setReviewed(event.target.checked)}
            />
            I reviewed all filenames, including other pages, and included every companion for this
            photo.
          </label>
          <button
            type="button"
            disabled={busy || !canConfirm || confirmation !== null}
            onClick={() => void run('confirm')}
          >
            {pending === 'confirm' ? 'Retaining companion review…' : 'Save companion review'}
          </button>
        </>
      )}
      {confirmation && (
        <section aria-label="Retained companion decision">
          <p role="status">
            Companion review retained · {confirmation.files.length}{' '}
            {confirmation.files.length === 1 ? 'file' : 'files'} ·{' '}
            {BigInt(confirmation.totalBytes).toLocaleString()} bytes. Copying remains unavailable.
          </p>
          <ul>
            {confirmation.files.map((file) => (
              <li key={file.entryId}>
                <bdi>{file.sourceName}</bdi> → <bdi>{file.targetRelativePath}</bdi>
              </li>
            ))}
          </ul>
          {onChoose && (
            <button type="button" disabled={busy || changed} onClick={() => onChoose(confirmation)}>
              Use this decision in copy review
            </button>
          )}
        </section>
      )}
      {history && (
        <section aria-label="Companion review history">
          <p>
            Retained decisions are not revalidated by loading history. Read all filenames again
            before any future operation.
          </p>
          {history.length === 0 ? (
            <p>No retained companion reviews.</p>
          ) : (
            <ul>
              {history.map((record) => (
                <li key={record.id}>
                  <button type="button" disabled={busy} onClick={() => setConfirmation(record)}>
                    View {record.files.length} files · review {record.id}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </section>
  );
}
