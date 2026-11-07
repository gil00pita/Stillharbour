import { Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  BundleConfirmation,
  BundleEntryKind,
  BundleInventory,
  BundleRelation,
  OrganizationRow,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

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
    <chakra.section
      className="organization-destination bundle-inventory"
      aria-label="Complete companion inventory"
      aria-busy={busy}
    >
      <Heading as="h3" unstyled ref={heading} tabIndex={-1}>
        Companions for <bdi>{row.sourcePath}</bdi>
      </Heading>
      <Text>
        Read all filenames in this photo’s immediate folder, including files excluded from the
        gallery. No file contents are read or changed. Folders, links and placeholders stay at
        source. This review creates no copy approval.
      </Text>
      <Button type="button" disabled={busy || changed} onClick={() => void run('inventory')}>
        {pending === 'inventory'
          ? 'Reading filenames…'
          : inventory
            ? 'Read filenames again'
            : 'Read all filenames'}
      </Button>
      <Button type="button" disabled={busy} onClick={() => void run('history')}>
        Load retained companion reviews
      </Button>
      <Button type="button" onClick={onClose}>
        Close companion inventory
      </Button>
      {error && (
        <Text className="error" role="alert">
          {error}
        </Text>
      )}
      {changed && (
        <Text role="alert">
          The retained plan or source changed. Build a fresh saved plan before confirming
          companions.
        </Text>
      )}
      {inventory && (
        <>
          <Text>
            {inventory.entries.length} observed entries ·{' '}
            {selected.size +
              (inventory.entries.some((entry) => entry.relation === 'primary') ? 1 : 0)}{' '}
            files selected for review
          </Text>
          {!inventory.inventoryComplete && (
            <Text role="alert">
              The inventory is incomplete. Truncated, unreadable or unobserved entries block
              confirmation.
            </Text>
          )}
          {(inventory.truncated || inventory.indexedLimitExceeded) && (
            <Text>Inventory limits were reached. This is not the complete folder scope.</Text>
          )}
          {inventory.unobservedCandidates.length > 0 && (
            <>
              <Text>Indexed candidates not observed in this folder:</Text>
              <List.Root unstyled>
                {inventory.unobservedCandidates.map((name) => (
                  <List.Item unstyled key={name}>
                    <bdi>{name}</bdi>
                  </List.Item>
                ))}
              </List.Root>
            </>
          )}
          <Box className="organization-table">
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top" ref={caption} tabIndex={-1}>
                All observed folder entries · page {page + 1} of{' '}
                {Math.max(1, Math.ceil(inventory.entries.length / PAGE_SIZE))}
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled scope="col">
                    Include
                  </Table.ColumnHeader>
                  <Table.ColumnHeader unstyled scope="col">
                    Name
                  </Table.ColumnHeader>
                  <Table.ColumnHeader unstyled scope="col">
                    Observation
                  </Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {inventory.entries.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE).map((entry) => (
                  <Table.Row unstyled key={entry.id}>
                    <Table.Cell unstyled>
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
                    </Table.Cell>
                    <Table.Cell unstyled>
                      <bdi>{entry.name}</bdi>
                      {!entry.nameRepresentable && (
                        <Text>Name cannot be represented by the template.</Text>
                      )}
                    </Table.Cell>
                    <Table.Cell unstyled>
                      {kinds[entry.kind]} · {relations[entry.relation]} ·{' '}
                      {entry.bytes === null
                        ? 'Size unavailable'
                        : `${BigInt(entry.bytes).toLocaleString()} bytes`}
                      <br />
                      {entry.catalogAssetId
                        ? `Catalog: ${entry.catalogPresence ?? 'unknown'}`
                        : 'Not indexed in the catalog'}
                      {entry.error && <Text>{entry.error}</Text>}
                    </Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Button
            type="button"
            disabled={busy || page === 0}
            onClick={() => setPage((value) => value - 1)}
          >
            Previous filenames
          </Button>
          <Button
            type="button"
            disabled={busy || (page + 1) * PAGE_SIZE >= inventory.entries.length}
            onClick={() => setPage((value) => value + 1)}
          >
            Next filenames
          </Button>
          <Text>
            Include every possible same-stem, filename-sidecar and AppleDouble companion.
            Unsupported or ambiguous companions block confirmation. You may also include other
            regular files explicitly. At most 64 companions can be retained per photo.
          </Text>
          {mandatoryMissing && (
            <Text>
              Some possible companions are excluded or cannot be included. This bundle cannot be
              confirmed.
            </Text>
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
          <Button
            type="button"
            disabled={busy || !canConfirm || confirmation !== null}
            onClick={() => void run('confirm')}
          >
            {pending === 'confirm' ? 'Retaining companion review…' : 'Save companion review'}
          </Button>
        </>
      )}
      {confirmation && (
        <chakra.section aria-label="Retained companion decision">
          <Text role="status">
            Companion review retained · {confirmation.files.length}{' '}
            {confirmation.files.length === 1 ? 'file' : 'files'} ·{' '}
            {BigInt(confirmation.totalBytes).toLocaleString()} bytes. Copying remains unavailable.
          </Text>
          <List.Root unstyled>
            {confirmation.files.map((file) => (
              <List.Item unstyled key={file.entryId}>
                <bdi>{file.sourceName}</bdi> → <bdi>{file.targetRelativePath}</bdi>
              </List.Item>
            ))}
          </List.Root>
          {onChoose && (
            <Button type="button" disabled={busy || changed} onClick={() => onChoose(confirmation)}>
              Use this decision in copy review
            </Button>
          )}
        </chakra.section>
      )}
      {history && (
        <chakra.section aria-label="Companion review history">
          <Text>
            Retained decisions are not revalidated by loading history. Read all filenames again
            before any future operation.
          </Text>
          {history.length === 0 ? (
            <Text>No retained companion reviews.</Text>
          ) : (
            <List.Root unstyled>
              {history.map((record) => (
                <List.Item unstyled key={record.id}>
                  <Button type="button" disabled={busy} onClick={() => setConfirmation(record)}>
                    View {record.files.length} files · review {record.id}
                  </Button>
                </List.Item>
              ))}
            </List.Root>
          )}
        </chakra.section>
      )}
    </chakra.section>
  );
}
