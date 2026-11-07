import { Field, Box, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type {
  DirectorySelection,
  OrganizationDestinationReview,
  OrganizationPreview,
  OrganizationPlanDetail,
  TemplateRef,
  OrganizationTargetState,
  LibraryRoot,
  ImportDestinationPreference,
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';

const targetLabels: Record<OrganizationTargetState, string> = {
  unoccupied_observed: 'No existing target observed',
  occupied: 'Target already exists — retained',
  ancestor_not_directory: 'A parent name is occupied by a file',
  unsupported_ancestor: 'A parent or package cannot be used',
  preview_blocked: 'Blocked in the template preview',
};
export type DestinationRootOption = Pick<LibraryRoot, 'id' | 'label'> &
  Partial<Pick<LibraryRoot, 'mode' | 'availability'>>;

export function OrganizationDestination({
  catalogId = null,
  preferredImportRoot = null,
  preview,
  revision,
  templateReference = null,
  roots = [],
}: {
  catalogId?: string | null;
  preferredImportRoot?: ImportDestinationPreference | null;
  preview: OrganizationPreview;
  revision: string;
  templateReference?: TemplateRef | null;
  roots?: DestinationRootOption[];
}) {
  const requestId = useRef(0);
  const [selection, setSelection] = useState<DirectorySelection | null>(null);
  const [review, setReview] = useState<OrganizationDestinationReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<OrganizationPlanDetail | null>(null);
  const preferredId =
    catalogId && preferredImportRoot?.catalogId === catalogId
      ? (preferredImportRoot.rootId ?? '')
      : '';
  const [destinationRootId, setDestinationRootId] = useState(preferredId);
  const selectedRoot = roots.find((root) => root.id === destinationRootId);
  const destinationProblem = !destinationRootId
    ? null
    : preview.rows.some((row) => row.rootId === destinationRootId)
      ? 'The preferred or selected destination is also a selected source. Choose a different destination.'
      : !selectedRoot || selectedRoot.availability !== 'online'
        ? 'The preferred or selected destination is unavailable. Reconnect it or explicitly choose another destination.'
        : selectedRoot.mode !== 'managed'
          ? 'The preferred or selected destination is read-only. Choose an authorized managed destination.'
          : null;
  useEffect(
    () => () => {
      requestId.current += 1;
    },
    [],
  );
  const stale =
    BigInt(preview.catalogRevision) < BigInt(revision) ||
    (review !== null &&
      (BigInt(review.catalogRevision) < BigInt(revision) ||
        review.previewFingerprint !== preview.fingerprint));
  async function check(choose: boolean) {
    if (busy || stale || destinationProblem) return;
    const id = ++requestId.current;
    setBusy(true);
    setError(null);
    try {
      const picked = choose ? await pickDirectory() : selection;
      if (requestId.current !== id || !picked) return;
      setSelection(picked);
      setSaved(null);
      setReview(null);
      if (!picked.identityAvailable)
        throw new Error(
          'This destination’s native identity cannot be verified. Choose a supported local folder.',
        );
      const result = await libraryData(
        {
          kind: 'organization_destination',
          selection_id: picked.id,
          asset_ids: preview.rows.map((row) => row.assetId),
          template: preview.template,
          preview_fingerprint: preview.fingerprint,
          ...(destinationRootId ? { destination_root_id: destinationRootId } : {}),
        },
        'organization_destination',
      );
      if (requestId.current === id) setReview(result);
    } catch (error) {
      if (requestId.current === id) {
        setReview(null);
        setError(errorMessage(error));
      }
    } finally {
      if (requestId.current === id) setBusy(false);
    }
  }
  async function save() {
    if (!selection || !review || stale || busy || saved || destinationProblem) return;
    const id = ++requestId.current;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        {
          kind: 'organization_plan_create',
          selection_id: selection.id,
          draft: {
            assetIds: preview.rows.map((row) => row.assetId),
            template: preview.template,
            previewFingerprint: preview.fingerprint,
            templateReference,
            ...(destinationRootId ? { destinationRootId } : {}),
          },
        },
        'organization_plan',
      );
      if (id === requestId.current) setSaved(result);
    } catch (error) {
      if (id === requestId.current) setError(errorMessage(error));
    } finally {
      if (id === requestId.current) setBusy(false);
    }
  }
  return (
    <chakra.section
      className="organization-destination"
      aria-label="Organization destination"
      aria-busy={busy}
    >
      <Heading as="h3" unstyled>
        Destination for organized copies
      </Heading>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Destination library policy</span>
          </Field.Label>
          <NativeSelect
            aria-label="Destination library policy"
            disabled={busy || stale}
            value={destinationRootId}
            onChange={(event) => {
              requestId.current += 1;
              setDestinationRootId(event.target.value);
              setSelection(null);
              setReview(null);
              setSaved(null);
              setError(null);
            }}
          >
            <option value="">Separate destination folder</option>
            {destinationRootId && !selectedRoot && (
              <option value={destinationRootId} disabled>
                Unavailable preferred destination
              </option>
            )}
            {roots.map((root) => {
              const source = preview.rows.some((row) => row.rootId === root.id);
              const reason = source
                ? 'selected source'
                : root.mode !== 'managed'
                  ? 'read-only'
                  : root.availability !== 'online'
                    ? 'unavailable'
                    : null;
              return (
                <option key={root.id} value={root.id} disabled={reason !== null}>
                  {root.label}
                  {reason ? ` — ${reason}` : ''}
                </option>
              );
            })}
          </NativeSelect>
        </label>
      </Field.Root>
      {preferredId && destinationRootId === preferredId && (
        <Text className="help">
          Saved preferred import destination for this catalog. This does not approve copying.
        </Text>
      )}
      {destinationProblem && <Text role="status">{destinationProblem}</Text>}
      <Text>
        {destinationRootId
          ? 'Choose the selected managed library’s root itself with the folder picker. Existing files are retained; new copies require a separately reviewed output namespace.'
          : 'Choose an existing folder outside every indexed library.'}{' '}
        Checking it leaves both folders unchanged.
      </Text>
      <Button
        type="button"
        disabled={busy || stale || !!destinationProblem}
        onClick={() => void check(true)}
      >
        {busy ? 'Checking destination…' : 'Choose destination'}
      </Button>
      {selection && (
        <>
          <Text>
            <bdi>{review?.destinationPath ?? selection.displayPath}</bdi>
          </Text>
          <Button
            type="button"
            disabled={busy || stale || !!destinationProblem}
            onClick={() => void check(false)}
          >
            Check destination again
          </Button>
        </>
      )}
      {error && (
        <Text role="alert" className="error">
          {error}
        </Text>
      )}
      {stale && (
        <Text role="alert">
          The catalog changed. Build a fresh preview before checking a destination.
        </Text>
      )}
      {review && !stale && (
        <>
          <Text role="status">Destination identity and indexed-root overlap checked.</Text>
          <Text>
            {BigInt(review.availableBytes).toLocaleString()} bytes available of{' '}
            {BigInt(review.totalBytes).toLocaleString()} total at review time. Selected photo bytes:{' '}
            {BigInt(preview.selectedBytes).toLocaleString()}. Companions, staging, retained versions
            and metadata reserves are not included; space is not reserved.
          </Text>
          <Box className="organization-table">
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top">
                Existing target review
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled scope="col">
                    Proposed relative path
                  </Table.ColumnHeader>
                  <Table.ColumnHeader unstyled scope="col">
                    Observation
                  </Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {review.targets.map((target) => (
                  <Table.Row unstyled key={target.assetId}>
                    <Table.Cell unstyled>
                      <bdi>{target.targetRelativePath ?? 'No proposed target'}</bdi>
                    </Table.Cell>
                    <Table.Cell unstyled>
                      {targetLabels[target.state]}
                      {target.blockingComponent && (
                        <>
                          <br />
                          <bdi>{target.blockingComponent}</bdi>
                        </>
                      )}
                    </Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Button
            type="button"
            disabled={busy || saved !== null || !!destinationProblem}
            onClick={() => void save()}
          >
            Save plan draft
          </Button>
        </>
      )}
      {saved && (
        <Text role="status">
          Plan draft saved. Reopen it from Saved plans. Copying remains unavailable.
        </Text>
      )}
      <Text className="help">
        Native lookup checks existing entries without following links. Missing parent folders and
        names are observations, not reservations. Equivalence between future case/Unicode names,
        complete companions, full space requirements and metadata fidelity still need review. Saving
        retains a draft without reserving target names. Copying remains unavailable.
      </Text>
    </chakra.section>
  );
}
