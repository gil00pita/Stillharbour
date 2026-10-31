import { useEffect, useRef, useState } from 'react';
import type {
  BundleConfirmation,
  CopyManifest,
  CopyManifestIssueKind,
  OrganizationPlanDetail,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';

const issues: Record<CopyManifestIssueKind, string> = {
  source_has_multiple_targets: 'A source has different proposed destinations across bundles',
  target_has_multiple_sources: 'Different sources have the same proposed destination',
  possible_case_collision: 'Proposed destinations may be equivalent under native case rules',
  file_directory_collision: 'A proposed file is also needed as a folder',
};
const PAGE_SIZE = 64;

export function CopyManifestReview({
  detail,
  choices,
}: {
  detail: OrganizationPlanDetail;
  choices: Record<string, BundleConfirmation>;
}) {
  const [manifest, setManifest] = useState<CopyManifest | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const request = useRef(0);
  const result = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  useEffect(() => {
    if (manifest) result.current?.focus();
  }, [manifest]);
  const decisionIds = detail.plan.preview.rows.map((row) => choices[row.assetId]?.id);
  const complete =
    decisionIds.length > 0 &&
    detail.plan.preview.rows.every(
      (row) =>
        choices[row.assetId]?.planId === detail.plan.id &&
        choices[row.assetId]?.assetId === row.assetId,
    );
  const changed =
    detail.stale ||
    (manifest !== null &&
      (manifest.planId !== detail.plan.id ||
        manifest.draftDigest !== detail.plan.digest ||
        manifest.bundleConfirmationIds.length !== decisionIds.length ||
        manifest.bundleConfirmationIds.some((id) => !decisionIds.includes(id))));
  async function compile() {
    if (pending || !complete || detail.stale) return;
    const token = ++request.current;
    setPending(true);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_manifest',
          input: {
            planId: detail.plan.id,
            bundleConfirmationIds: decisionIds.filter((id): id is string => id !== undefined),
          },
        },
        'organization_copy_manifest',
      );
      if (request.current !== token) return;
      if (value.planId !== detail.plan.id || value.draftDigest !== detail.plan.digest)
        throw new Error('The copy review belongs to a different draft. Reopen the saved plan.');
      setManifest(value);
      setPage(0);
    } catch (error) {
      if (request.current === token) setError(errorMessage(error));
    } finally {
      if (request.current === token) setPending(false);
    }
  }
  const pages = manifest ? Math.max(1, Math.ceil(manifest.files.length / PAGE_SIZE)) : 1;
  return (
    <section
      className="organization-destination"
      aria-label="Compiled copy review"
      aria-busy={pending}
    >
      <h3>Review all selected files together</h3>
      <p>
        Choose a companion decision for each photo. Compilation rechecks those exact inventories and
        combines identical source-to-target mappings. Conflicting mappings stay blocked for review.
      </p>
      <p>
        {decisionIds.filter(Boolean).length} of {decisionIds.length} photos have an explicitly
        chosen decision.
      </p>
      <button
        type="button"
        disabled={pending || !complete || detail.stale}
        onClick={() => void compile()}
      >
        {pending ? 'Compiling copy review…' : 'Compile copy review'}
      </button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {manifest && (
        <>
          <h4 ref={result} tabIndex={-1}>
            Compiled file manifest
          </h4>
          {changed && (
            <p role="alert">The draft or chosen companion scope changed. Compile a fresh review.</p>
          )}
          <p>
            {manifest.files.length} proposed file outputs ·{' '}
            {BigInt(manifest.selectedBytes).toLocaleString()} bytes. Identical overlapping mappings
            are counted once.
          </p>
          <p>
            Native target equivalence, capacity, metadata fidelity, content verification and
            executor recovery remain pending. This retained manifest does not approve copying.
          </p>
          {manifest.issues.length > 0 && (
            <p role="alert">
              {manifest.issues.length} conflicts block the proposed copy. Build a new draft or
              choose a compatible companion scope.
            </p>
          )}
          {manifest.issues.length > 0 && (
            <ul>
              {manifest.issues.slice(0, 64).map((issue, index) => (
                <li key={index}>
                  {issues[issue.kind]} ({issue.fileIds.length} files)
                </li>
              ))}
            </ul>
          )}
          {manifest.issues.length > 64 && (
            <p>{manifest.issues.length - 64} additional conflicts are retained in the manifest.</p>
          )}
          <div className="organization-table">
            <table>
              <caption>
                Compiled files, page {page + 1} of {pages}
              </caption>
              <thead>
                <tr>
                  <th>Source filename</th>
                  <th>Proposed destination</th>
                  <th>Bytes</th>
                  <th>Photo bundles</th>
                </tr>
              </thead>
              <tbody>
                {manifest.files.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE).map((file) => (
                  <tr key={file.id}>
                    <td>
                      <bdi title={`Library ${file.rootId}`}>{file.sourceName}</bdi>
                    </td>
                    <td>
                      <bdi>{file.targetRelativePath}</bdi>
                    </td>
                    <td>{BigInt(file.bytes).toLocaleString()}</td>
                    <td>{file.bundleAssetIds.length}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button type="button" disabled={page === 0} onClick={() => setPage((value) => value - 1)}>
            Previous compiled files
          </button>
          <button
            type="button"
            disabled={page + 1 >= pages}
            onClick={() => setPage((value) => value + 1)}
          >
            Next compiled files
          </button>
          <p className="help">Manifest fingerprint: {manifest.digest}</p>
        </>
      )}
    </section>
  );
}
