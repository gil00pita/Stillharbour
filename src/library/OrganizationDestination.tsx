import { useEffect, useRef, useState } from 'react';
import type {
  DirectorySelection,
  OrganizationDestinationReview,
  OrganizationPreview,
  OrganizationPlanDetail,
  TemplateRef,
  OrganizationTargetState,
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';

const targetLabels: Record<OrganizationTargetState, string> = {
  unoccupied_observed: 'No existing target observed',
  occupied: 'Target already exists — retained',
  ancestor_not_directory: 'A parent name is occupied by a file',
  unsupported_ancestor: 'A parent or package cannot be used',
  preview_blocked: 'Blocked in the template preview',
};

export function OrganizationDestination({
  preview,
  revision,
  templateReference = null,
}: {
  preview: OrganizationPreview;
  revision: string;
  templateReference?: TemplateRef | null;
}) {
  const requestId = useRef(0);
  const [selection, setSelection] = useState<DirectorySelection | null>(null);
  const [review, setReview] = useState<OrganizationDestinationReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<OrganizationPlanDetail | null>(null);
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
    if (busy || stale) return;
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
    if (!selection || !review || stale || busy || saved) return;
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
    <section
      className="organization-destination"
      aria-label="Organization destination"
      aria-busy={busy}
    >
      <h3>Destination for organized copies</h3>
      <p>
        Choose an existing folder outside every indexed library. Checking it leaves both folders
        unchanged.
      </p>
      <button type="button" disabled={busy || stale} onClick={() => void check(true)}>
        {busy ? 'Checking destination…' : 'Choose destination'}
      </button>
      {selection && (
        <>
          <p>
            <bdi>{review?.destinationPath ?? selection.displayPath}</bdi>
          </p>
          <button type="button" disabled={busy || stale} onClick={() => void check(false)}>
            Check destination again
          </button>
        </>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {stale && (
        <p role="alert">
          The catalog changed. Build a fresh preview before checking a destination.
        </p>
      )}
      {review && !stale && (
        <>
          <p role="status">Destination identity and indexed-root overlap checked.</p>
          <p>
            {BigInt(review.availableBytes).toLocaleString()} bytes available of{' '}
            {BigInt(review.totalBytes).toLocaleString()} total at review time. Selected photo bytes:{' '}
            {BigInt(preview.selectedBytes).toLocaleString()}. Companions, staging, retained versions
            and metadata reserves are not included; space is not reserved.
          </p>
          <div className="organization-table">
            <table>
              <caption>Existing target review</caption>
              <thead>
                <tr>
                  <th scope="col">Proposed relative path</th>
                  <th scope="col">Observation</th>
                </tr>
              </thead>
              <tbody>
                {review.targets.map((target) => (
                  <tr key={target.assetId}>
                    <td>
                      <bdi>{target.targetRelativePath ?? 'No proposed target'}</bdi>
                    </td>
                    <td>
                      {targetLabels[target.state]}
                      {target.blockingComponent && (
                        <>
                          <br />
                          <bdi>{target.blockingComponent}</bdi>
                        </>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button type="button" disabled={busy || saved !== null} onClick={() => void save()}>
            Save plan draft
          </button>
        </>
      )}
      {saved && (
        <p role="status">
          Plan draft saved. Reopen it from Saved plans. Copying remains unavailable.
        </p>
      )}
      <p className="help">
        Native lookup checks existing entries without following links. Missing parent folders and
        names are observations, not reservations. Equivalence between future case/Unicode names,
        complete companions, full space requirements and metadata fidelity still need review. Saving
        retains a draft without reserving target names. Copying remains unavailable.
      </p>
    </section>
  );
}
