import { useEffect, useRef, useState } from 'react';
import type {
  CopyManifest,
  CopyActionReview,
  CopyActionApproval,
  CopyWorkflowJob,
  DirectorySelection,
  CopyInspection,
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';
import { CopyJobProgress, CopyIoControls } from './CopyJobProgress';
import { copyJobActive } from './copyJobs';

function validateReview(value: CopyActionReview, plan: string, manifest: string) {
  if (
    value.planId !== plan ||
    value.manifestId !== manifest ||
    !value.sourcesRetained ||
    value.independentBackup ||
    !value.files.length ||
    value.files.some((file) => !file.targetPath.startsWith(`${value.outputFolder}/`))
  ) {
    throw new Error(
      'The native review does not match these selected files. Reopen the saved plan.',
    );
  }
}
export function VerifiedCopyWorkflow({ manifest }: { manifest: CopyManifest }) {
  const [selection, setSelection] = useState<DirectorySelection | null>(null);
  const [review, setReview] = useState<CopyActionReview | null>(null);
  const [approval, setApproval] = useState<CopyActionApproval | null>(null);
  const [job, setJob] = useState<CopyWorkflowJob | null>(null);
  const [inspection, setInspection] = useState<CopyInspection | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [readyRetry, setReadyRetry] = useState(0);
  const epoch = useRef(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  useEffect(() => {
    if (review) heading.current?.focus();
  }, [review]);
  const running = job !== null && copyJobActive(job);
  const jobId = job?.id;
  const selectionId = selection?.id;
  useEffect(() => {
    if (!running || !jobId || !selectionId) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const current = await libraryData(
          { kind: 'organization_copy_job', job_id: jobId! },
          'organization_copy_job',
        );
        if (!active) return;
        if (current.id !== jobId || current.planId !== manifest.planId)
          throw new Error('The native job belongs to another operation.');
        setJob(current);
        if (copyJobActive(current)) timer = setTimeout(() => void poll(), 250);
      } catch (error) {
        if (active) {
          setError(errorMessage(error));
          timer = setTimeout(() => void poll(), 1000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 100);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [running, jobId, selectionId, manifest.planId, manifest.id]);
  const prepared = job?.state === 'prepared';
  const preparedReview = job?.readyReviewId;
  const preparationReview = job?.reviewId;
  useEffect(() => {
    if (!prepared || !preparedReview || !preparationReview || !selectionId) return;
    let active = true;
    libraryData(
      { kind: 'organization_copy_review', review_id: preparationReview, selection_id: selectionId },
      'organization_copy_review',
    )
      .then((value) => {
        if (!active) return;
        validateReview(value, manifest.planId, manifest.id);
        if (value.id !== preparedReview || value.action !== 'copy_files')
          throw new Error('The prepared copy review does not match its native job.');
        setReview(value);
        setApproval(null);
        setAcknowledged(false);
        setPage(0);
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [
    prepared,
    preparedReview,
    preparationReview,
    selectionId,
    manifest.planId,
    manifest.id,
    readyRetry,
  ]);
  async function create() {
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    try {
      const capabilities = await libraryData(
        { kind: 'organization_copy_capabilities' },
        'organization_copy_capabilities',
      );
      if (epoch.current !== token) return;
      if (!capabilities.available) {
        setUnavailable(capabilities.reason);
        return;
      }
      const chosen = await pickDirectory();
      if (epoch.current !== token || !chosen) return;
      if (!chosen.identityAvailable)
        throw new Error(
          'The selected folder identity cannot be verified. Choose a supported local destination.',
        );
      const value = await libraryData(
        {
          kind: 'organization_copy_review_create',
          plan_id: manifest.planId,
          manifest_id: manifest.id,
          selection_id: chosen.id,
        },
        'organization_copy_review',
      );
      if (epoch.current !== token) return;
      validateReview(value, manifest.planId, manifest.id);
      setSelection(chosen);
      setReview(value);
      setApproval(null);
      setAcknowledged(false);
      setInspection(null);
      setPage(0);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  async function approve() {
    if (!review || !selection) return;
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_approve',
          selection_id: selection.id,
          input: {
            reviewId: review.id,
            digest: review.digest,
            action: review.action,
            fidelityLimitsAcknowledged: acknowledged,
          },
        },
        'organization_copy_approval',
      );
      if (epoch.current !== token) return;
      if (value.reviewId !== review.id || value.action !== review.action)
        throw new Error('The approval belongs to another review. Review again.');
      setApproval(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  async function start() {
    if (!review || !selection || !approval) return;
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    const ticket = approval.id;
    setApproval(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_start',
          selection_id: selection.id,
          input: { reviewId: review.id, digest: review.digest, approvalId: ticket },
        },
        'organization_copy_job',
      );
      if (epoch.current !== token) return;
      if (
        value.reviewId !== review.id ||
        value.planId !== manifest.planId ||
        value.action !== review.action
      )
        throw new Error('The queued job belongs to another review. Check Jobs before retrying.');
      setJob(value);
      setInspection(null);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  async function inspect() {
    if (!review || !selection) return;
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    try {
      const value = await libraryData(
        { kind: 'organization_copy_inspect', review_id: review.id, selection_id: selection.id },
        'organization_copy_inspection',
      );
      if (epoch.current !== token) return;
      if (value.reviewId !== review.id)
        throw new Error('The inspection belongs to another copy review.');
      setInspection(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  const blocked =
    busy ||
    running ||
    job?.state === 'completed_verified' ||
    job?.state === 'needs_recovery' ||
    job?.state === 'needs_attention' ||
    job?.state === 'canceled_with_partial_copies' ||
    (job?.state === 'prepared' && review?.action === 'prepare_names');
  const pages = review ? Math.max(1, Math.ceil(review.files.length / 64)) : 1;
  return (
    <section
      className="organization-destination"
      aria-label="Verified organization copy"
      aria-busy={busy}
    >
      <h3>Prepare a verified copy</h3>
      <p>
        Confirm the saved destination using the native picker. Review the new output folder before
        preparing names. Preparation creates empty staged names; copying needs a separate approval.
        Originals remain in place.
      </p>
      {!review && (
        <button type="button" disabled={busy || !!unavailable} onClick={() => void create()}>
          {busy ? 'Reviewing native scope…' : 'Choose destination and review preparation'}
        </button>
      )}
      {unavailable && <p role="status">{unavailable}</p>}
      {prepared && review?.action === 'prepare_names' && (
        <button type="button" onClick={() => setReadyRetry(readyRetry + 1)}>
          Reload prepared copy review
        </button>
      )}
      {review && (
        <>
          <h4 ref={heading} tabIndex={-1}>
            {review.action === 'prepare_names'
              ? 'Output preparation review'
              : 'Prepared copy review'}
          </h4>
          <p>
            Destination: <bdi>{review.destinationDisplayPath}</bdi>
          </p>
          <p>
            New output folder:{' '}
            <strong>
              <bdi>{review.outputFolder}</bdi>
            </strong>
          </p>
          <p>
            {review.files.length} selected files ·{' '}
            {BigInt(review.assessment.capacity.logicalBytes).toLocaleString()} bytes. Sources
            retained; no independent backup is provided.
          </p>
          <p>
            Estimated output space:{' '}
            {BigInt(review.assessment.capacity.estimatedPeakAdditionalBytes).toLocaleString()}{' '}
            bytes. Space is not reserved. The executor also requires application-journal headroom
            before starting.
          </p>
          <div className="organization-table">
            <table>
              <caption>
                Exact output paths, page {page + 1} of {pages}
              </caption>
              <thead>
                <tr>
                  <th>Source relative path</th>
                  <th>Output relative path</th>
                  <th>Bytes</th>
                </tr>
              </thead>
              <tbody>
                {review.files.slice(page * 64, (page + 1) * 64).map((file) => (
                  <tr key={file.id}>
                    <td>
                      <bdi>{file.sourcePath}</bdi>
                    </td>
                    <td>
                      <bdi>{file.targetPath}</bdi>
                    </td>
                    <td>{BigInt(file.bytes).toLocaleString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <button type="button" disabled={!page} onClick={() => setPage(page - 1)}>
            Previous output paths
          </button>
          <button type="button" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)}>
            Next output paths
          </button>
          {review.action === 'copy_files' && (
            <label>
              <input
                type="checkbox"
                checked={acknowledged}
                disabled={blocked || !!approval}
                onChange={(event) => setAcknowledged(event.target.checked)}
              />{' '}
              I understand that file bytes and selected companions are verified, while timestamps,
              permissions, extended attributes/resource forks, alternate streams and
              sparse/compression layout are not preserved or verified. Originals remain.
            </label>
          )}
          {!approval ? (
            <button
              type="button"
              disabled={blocked || (review.action === 'copy_files' && !acknowledged)}
              onClick={() => void approve()}
            >
              {review.action === 'prepare_names'
                ? 'Approve output preparation'
                : 'Approve byte-verified copy'}
            </button>
          ) : (
            <>
              <p>Approved for one attempt in this session; expires after ten minutes.</p>
              <button type="button" disabled={blocked} onClick={() => void start()}>
                {review.action === 'prepare_names'
                  ? 'Start output preparation'
                  : 'Start verified copy'}
              </button>
            </>
          )}
          {review.action === 'copy_files' && !running && (
            <button type="button" disabled={busy} onClick={() => void inspect()}>
              Inspect actual output bytes
            </button>
          )}
        </>
      )}
      {job && (
        <CopyJobProgress
          job={job}
          onRefresh={async () =>
            setJob(
              await libraryData(
                { kind: 'organization_copy_job', job_id: job.id },
                'organization_copy_job',
              ),
            )
          }
        />
      )}
      {running && job?.action === 'copy_files' && <CopyIoControls />}
      {inspection && (
        <p role="status">
          {inspection.allPublishedBytesVerified
            ? 'Every selected output was freshly compared with its source.'
            : `Inspection: ${inspection.states.join(', ')}. No writes were replayed.`}
        </p>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
