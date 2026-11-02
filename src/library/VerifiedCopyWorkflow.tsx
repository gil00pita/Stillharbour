import { Button } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import type {
  CopyManifest,
  CopyActionReview,
  CopyActionApproval,
  CopyWorkflowJob,
  DirectorySelection,
  CopyInspection,
  CopyEffectMatch,
  ExistingCopyEffect,
  CopyPreparationReopened,
  ImportPlan,
  ImportCopyReceipt,
} from '../bindings';
import { validateImportCopyReview, validateImportCopyReceipt } from './importPlanValidation';
import { errorMessage, library, libraryData, pickDirectory } from '../ipc';
import { CopyJobProgress, CopyIoControls } from './CopyJobProgress';
import { copyJobActive } from './copyJobs';
import { RetainedCopyInspection } from './RetainedCopyInspection';
import { ImportOutputIndexReview } from './ImportOutputIndexReview';
import { RootPartitionCopyWorkflow } from './RootPartitionCopyWorkflow';

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
type PreparedReview = { review: CopyActionReview; selection: DirectorySelection };
function validateReopened(value: CopyPreparationReopened, job: CopyWorkflowJob) {
  if (value.importPlan) validateImportCopyReview(value.review, value.manifest, value.importPlan);
  const files = value.importPlan
    ? value.manifest.files.filter((file) => value.importPlan!.copyFileIds.includes(file.id))
    : value.manifest.files.filter((file) => file.rootId === job.sourceRootId);
  if (
    value.jobId !== job.id ||
    value.manifest.planId !== job.planId ||
    value.review.sourceRootId !== job.sourceRootId ||
    value.review.action !== 'copy_files' ||
    value.review.files.length !== job.totalFiles ||
    files.length !== job.totalFiles ||
    files.some((file, index) => {
      const observed = value.review.files[index];
      return (
        !observed ||
        file.id !== observed.id ||
        file.rootId !== job.sourceRootId ||
        file.bytes !== observed.bytes ||
        observed.targetPath !== `${value.review.outputFolder}/${file.targetRelativePath}`
      );
    })
  )
    throw new Error('The reopened preparation does not match this job.');
  validateReview(value.review, job.planId, value.manifest.id);
}
type WorkflowProps = {
  manifest: CopyManifest;
  initial?: PreparedReview | undefined;
  importPlan?: ImportPlan;
  onActivityChange?: ((active: boolean) => void) | undefined;
};
export function VerifiedCopyWorkflow(props: WorkflowProps) {
  if (new Set(props.manifest.files.map((file) => file.rootId)).size > 1 && !props.importPlan)
    return (
      <RootPartitionCopyWorkflow
        manifest={props.manifest}
        initialRootId={props.initial?.review.sourceRootId}
        onActivityChange={props.onActivityChange}
      >
        {(root, activity) => (
          <CopyWorkflowSession
            key={root}
            {...props}
            initial={props.initial?.review.sourceRootId === root ? props.initial : undefined}
            sourceRootId={root}
            onActivityChange={activity}
          />
        )}
      </RootPartitionCopyWorkflow>
    );
  return <CopyWorkflowSession {...props} />;
}
function validatePartition(
  value: CopyActionReview,
  manifest: CopyManifest,
  root: string | undefined,
) {
  if (!root) return;
  const files = manifest.files.filter((file) => file.rootId === root);
  if (
    value.sourceRootId !== root ||
    value.files.length !== files.length ||
    files.some((file, index) => {
      const observed = value.files[index];
      return (
        !observed ||
        observed.id !== file.id ||
        observed.bytes !== file.bytes ||
        observed.targetPath !== `${value.outputFolder}/${file.targetRelativePath}`
      );
    })
  )
    throw new Error(
      'The native review does not match this source library. Review the selected library again.',
    );
}
function CopyWorkflowSession({
  manifest,
  initial,
  importPlan,
  onActivityChange,
  sourceRootId,
}: WorkflowProps & { sourceRootId?: string }) {
  const [selection, setSelection] = useState<DirectorySelection | null>(initial?.selection ?? null);
  const [existing, setExisting] = useState<CopyEffectMatch | null>(null);
  const [review, setReview] = useState<CopyActionReview | null>(initial?.review ?? null);
  const [approval, setApproval] = useState<CopyActionApproval | null>(null);
  const [job, setJob] = useState<CopyWorkflowJob | null>(null);
  const [inspection, setInspection] = useState<CopyInspection | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [readyRetry, setReadyRetry] = useState(0);
  const [receipt, setReceipt] = useState<ImportCopyReceipt | null>(null);
  const [receiptRetry, setReceiptRetry] = useState(0);
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
  useEffect(() => {
    onActivityChange?.(busy || running);
    return () => onActivityChange?.(false);
  }, [busy, running, onActivityChange]);
  const lifetime = useRef({ review, job });
  useEffect(() => {
    lifetime.current = { review, job };
  }, [review, job]);
  useEffect(
    () => () => {
      if (!importPlan && !sourceRootId) return;
      const current = lifetime.current;
      if (current.job && copyJobActive(current.job)) {
        if (importPlan)
          void library({ kind: 'organization_copy_cancel', job_id: current.job.id }).catch(
            () => {},
          );
      } else if (current.review)
        void library({ kind: 'organization_copy_discard', review_id: current.review.id }).catch(
          () => {},
        );
    },
    [importPlan, sourceRootId],
  );
  useEffect(() => {
    if (!importPlan || !job || job.state !== 'completed_verified') return;
    let active = true;
    libraryData(
      { kind: 'organization_import_copy_receipt', job_id: job.id },
      'organization_import_copy_receipt',
    )
      .then((value) => {
        if (!active) return;
        if (!value)
          throw new Error(
            'No completed import receipt is available. Retain output and review job history.',
          );
        validateImportCopyReceipt(value, importPlan, job);
        setReceipt(value);
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      });
    return () => {
      active = false;
    };
  }, [importPlan, job, receiptRetry]);
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
        validatePartition(value, manifest, sourceRootId);
        if (importPlan) validateImportCopyReview(value, manifest, importPlan);
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
    manifest,
    importPlan,
    readyRetry,
    sourceRootId,
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
      if (!importPlan) {
        const matched = await libraryData(
          {
            kind: 'organization_copy_effect_match',
            plan_id: manifest.planId,
            manifest_id: manifest.id,
            selection_id: chosen.id,
            ...(sourceRootId ? { source_root_id: sourceRootId } : {}),
          },
          'organization_copy_effect_match',
        );
        if (epoch.current !== token) return;
        if (
          matched.planId !== manifest.planId ||
          matched.manifestId !== manifest.id ||
          !/^[a-f0-9]{64}$/.test(matched.digest) ||
          matched.existing.length > 64 ||
          matched.existing.some(
            (entry) =>
              !entry.outputFolder.startsWith('Stillharbour organized ') ||
              (entry.job && entry.job.planId !== entry.planId),
          )
        )
          throw new Error('Existing copy history does not match this saved scope.');
        if (!matched.canReuseReview) {
          if (!matched.existing.length)
            throw new Error('Existing output history is incomplete. Reopen the saved plan.');
          setExisting(matched);
          setSelection(chosen);
          return;
        }
        if (
          matched.existing.length &&
          (matched.existing.length !== 1 ||
            matched.existing[0]!.planId !== manifest.planId ||
            matched.existing[0]!.manifestId !== manifest.id ||
            matched.existing[0]!.preparationRecorded ||
            matched.existing[0]!.job)
        )
          throw new Error('Existing preparation must be inspected before continuing.');
        setExisting(null);
      }
      const value = await libraryData(
        importPlan
          ? {
              kind: 'organization_import_copy_review',
              plan_id: manifest.planId,
              import_plan_id: importPlan.id,
              selection_id: chosen.id,
            }
          : {
              kind: 'organization_copy_review_create',
              plan_id: manifest.planId,
              manifest_id: manifest.id,
              selection_id: chosen.id,
              ...(sourceRootId ? { source_root_id: sourceRootId } : {}),
            },
        'organization_copy_review',
      );
      validateReview(value, manifest.planId, manifest.id);
      validatePartition(value, manifest, sourceRootId);
      if (importPlan) validateImportCopyReview(value, manifest, importPlan);
      if (epoch.current !== token) {
        if (importPlan || sourceRootId)
          void library({ kind: 'organization_copy_discard', review_id: value.id }).catch(() => {});
        return;
      }
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
  async function reopen() {
    if (!job || job.action !== 'prepare_names') return;
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    setApproval(null);
    setAcknowledged(false);
    try {
      const chosen = await pickDirectory();
      if (epoch.current !== token || !chosen) return;
      if (!chosen.identityAvailable)
        throw new Error('The selected folder identity is unavailable.');
      const value = await libraryData(
        { kind: 'organization_copy_preparation_reopen', job_id: job.id, selection_id: chosen.id },
        'organization_copy_preparation_reopened',
      );
      if (epoch.current !== token) {
        if (value.jobId === job.id && value.review.planId === job.planId)
          void library({ kind: 'organization_copy_discard', review_id: value.review.id }).catch(
            () => {},
          );
        return;
      }
      validateReopened(value, job);
      if (
        importPlan &&
        (!value.importPlan ||
          value.importPlan.id !== importPlan.id ||
          value.importPlan.digest !== importPlan.digest)
      )
        throw new Error('The reopened preparation belongs to different import decisions.');
      validateReview(value.review, manifest.planId, manifest.id);
      validatePartition(value.review, manifest, sourceRootId);
      setSelection(chosen);
      setReview(value.review);
      setJob(null);
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
      if (value.reviewId !== review.id || value.action !== review.action)
        throw new Error('The approval belongs to another review. Review again.');
      if (epoch.current !== token) {
        if (importPlan)
          void library({ kind: 'organization_copy_discard', review_id: review.id }).catch(() => {});
        return;
      }
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
      if (
        value.reviewId !== review.id ||
        value.planId !== manifest.planId ||
        value.action !== review.action
      )
        throw new Error('The queued job belongs to another review. Check Jobs before retrying.');
      if (epoch.current !== token) {
        if (importPlan)
          void library({ kind: 'organization_copy_cancel', job_id: value.id }).catch(() => {});
        return;
      }
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
      {importPlan && (
        <p>
          {importPlan.copyFileIds.length} new files will be copied. Every selected existing member
          is freshly byte-compared before and after each action. Existing files and originals remain
          in place.
        </p>
      )}
      <p>
        Confirm the saved destination using the native picker. Review the new output folder before
        preparing names. Preparation creates empty staged names; copying needs a separate approval.
        Originals remain in place.
      </p>
      {!review && (
        <Button type="button" disabled={busy || !!unavailable} onClick={() => void create()}>
          {busy ? 'Reviewing native scope…' : 'Choose destination and review preparation'}
        </Button>
      )}
      {unavailable && <p role="status">{unavailable}</p>}
      {receipt && (
        <p role="status">
          Import recorded: {receipt.copiedFileIds.length} new{' '}
          {receipt.copiedFileIds.length === 1 ? 'file' : 'files'} copied and{' '}
          {receipt.skippedFiles.length} existing{' '}
          {receipt.skippedFiles.length === 1 ? 'file' : 'files'} freshly verified. Sources retained.
        </p>
      )}
      {receipt && (
        <ImportOutputIndexReview receipt={receipt} sourceRootId={manifest.files[0]!.rootId} />
      )}
      {importPlan && job?.state === 'completed_verified' && !receipt && (
        <Button type="button" onClick={() => setReceiptRetry((old) => old + 1)}>
          Reload import receipt
        </Button>
      )}
      {existing && (
        <section aria-label="Existing copy output history">
          <h4>Existing output for this mapping</h4>
          <p>
            This source, template and target mapping already has retained output history. Inspect
            its current bytes before deciding whether it is complete. A new draft will reuse this
            history.
          </p>
          {existing.existing.map((entry) => (
            <ExistingEffect key={entry.reviewId} entry={entry} />
          ))}
        </section>
      )}
      {job?.action === 'prepare_names' && prepared && review?.action === 'copy_files' && (
        <Button type="button" disabled={busy} onClick={() => void reopen()}>
          Choose destination and reopen prepared review
        </Button>
      )}
      {prepared && review?.action === 'prepare_names' && (
        <Button type="button" onClick={() => setReadyRetry(readyRetry + 1)}>
          Reload prepared copy review
        </Button>
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
            {review.files.length} selected {review.files.length === 1 ? 'file' : 'files'} ·{' '}
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
          <Button type="button" disabled={!page} onClick={() => setPage(page - 1)}>
            Previous output paths
          </Button>
          <Button type="button" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)}>
            Next output paths
          </Button>
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
            <Button
              type="button"
              disabled={blocked || (review.action === 'copy_files' && !acknowledged)}
              onClick={() => void approve()}
            >
              {review.action === 'prepare_names'
                ? 'Approve output preparation'
                : 'Approve byte-verified copy'}
            </Button>
          ) : (
            <>
              <p>Approved for one attempt in this session; expires after ten minutes.</p>
              <Button type="button" disabled={blocked} onClick={() => void start()}>
                {review.action === 'prepare_names'
                  ? 'Start output preparation'
                  : 'Start verified copy'}
              </Button>
            </>
          )}
          {review.action === 'copy_files' && !running && (
            <Button type="button" disabled={busy} onClick={() => void inspect()}>
              Inspect actual output bytes
            </Button>
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

function ExistingEffect({ entry }: { entry: ExistingCopyEffect }) {
  const [original, setOriginal] = useState<CopyManifest | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  async function open() {
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_manifest_open',
          plan_id: entry.planId,
          manifest_id: entry.manifestId,
        },
        'organization_copy_manifest',
      );
      if (epoch.current !== token) return;
      if (value.planId !== entry.planId || value.id !== entry.manifestId)
        throw new Error('The saved mapping belongs to another operation.');
      setOriginal(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  return (
    <article>
      <p>
        Retained output folder: <bdi>{entry.outputFolder}</bdi>
      </p>
      {entry.job ? (
        entry.job.action === 'prepare_names' && !entry.job.readyReviewId ? (
          <ReopenPreparedCopy job={entry.job} />
        ) : (
          <RetainedCopyInspection job={entry.job} />
        )
      ) : entry.preparationRecorded ? (
        <p>
          Preparation evidence exists without a linked job. Preserve the retained folder and review
          its journal; automatic replay is unavailable.
        </p>
      ) : original ? (
        <VerifiedCopyWorkflow manifest={original} />
      ) : (
        <Button type="button" disabled={busy} onClick={() => void open()}>
          Review original saved mapping
        </Button>
      )}
      {error && <p role="alert">{error}</p>}
    </article>
  );
}

export function ReopenPreparedCopy({ job }: { job: CopyWorkflowJob }) {
  const [opened, setOpened] = useState<
    (CopyPreparationReopened & { selection: DirectorySelection }) | null
  >(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [job.id],
  );
  async function reopen() {
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    try {
      const capability = await libraryData(
        { kind: 'organization_copy_capabilities' },
        'organization_copy_capabilities',
      );
      if (epoch.current !== token) return;
      if (!capability.available) throw new Error(capability.reason);
      const selection = await pickDirectory();
      if (epoch.current !== token || !selection) return;
      if (!selection.identityAvailable)
        throw new Error('The selected folder identity is unavailable.');
      const value = await libraryData(
        {
          kind: 'organization_copy_preparation_reopen',
          job_id: job.id,
          selection_id: selection.id,
        },
        'organization_copy_preparation_reopened',
      );
      if (epoch.current !== token) {
        if (value.jobId === job.id && value.review.planId === job.planId)
          void library({ kind: 'organization_copy_discard', review_id: value.review.id }).catch(
            () => {},
          );
        return;
      }
      validateReopened(value, job);
      setOpened({ ...value, selection });
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setBusy(false);
    }
  }
  return (
    <section aria-label="Reopen retained preparation">
      <p>
        A complete retained preparation can be reopened after its original empty objects are
        validated. New fidelity acknowledgment, copy approval and start are required. Incomplete or
        changed objects remain retained and blocked.
      </p>
      {opened ? (
        <VerifiedCopyWorkflow
          key={opened.review.id}
          manifest={opened.manifest}
          initial={{ review: opened.review, selection: opened.selection }}
          {...(opened.importPlan ? { importPlan: opened.importPlan } : {})}
        />
      ) : (
        <Button type="button" disabled={busy} onClick={() => void reopen()}>
          {busy ? 'Checking retained preparation…' : 'Choose destination and reopen preparation'}
        </Button>
      )}
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
