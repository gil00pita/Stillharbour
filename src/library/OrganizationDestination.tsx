import { useEffect, useRef, useState } from 'react';
import type {
  DirectorySelection,
  OrganizationDestinationReview,
  OrganizationPreview,
  OrganizationPlanDetail,
  TemplateRef,
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';

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
        Existing target names, case and Unicode rules, companion inventory, capacity and metadata
        fidelity still need review. Saving retains a draft without reserving target names. Copying
        remains unavailable.
      </p>
    </section>
  );
}
