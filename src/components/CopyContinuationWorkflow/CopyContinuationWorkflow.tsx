import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  CopyActionApproval,
  CopyContinuationReview,
  CopyRecordedPhase,
  CopyWorkflowJob,
  DirectorySelection,
} from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { CopyIoControls, CopyJobProgress } from '@/components/CopyJobProgress';
import { copyJobActive } from '../../library/copyJobs';

const phases: Record<CopyRecordedPhase, string> = {
  not_started: 'Not started',
  intent_recorded: 'Intent recorded',
  stage_recorded: 'Stage recorded',
  stage_flushed: 'Stage flush recorded',
  stage_verified: 'Stage verification recorded',
  publication_intent: 'Publication intent recorded',
  completed_recorded: 'Completion recorded',
};
export function CopyContinuationWorkflow({
  job,
  selection,
  onRefresh,
}: {
  job: CopyWorkflowJob;
  selection: DirectorySelection;
  onRefresh?: () => Promise<void>;
}) {
  const [review, setReview] = useState<CopyContinuationReview | null>(null);
  const [approval, setApproval] = useState<CopyActionApproval | null>(null);
  const [current, setCurrent] = useState<CopyWorkflowJob | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const epoch = useRef(0);
  const retainedReview = useRef<string | null>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      epoch.current += 1;
      if (retainedReview.current)
        void library({
          kind: 'organization_copy_continuation_discard',
          review_id: retainedReview.current,
        }).catch(() => {});
    },
    [],
  );
  useEffect(() => {
    if (review) heading.current?.focus();
  }, [review]);
  const active = current !== null && copyJobActive(current);
  const id = current?.id;
  const plan = review?.planId;
  const stream = job.action === 'copy_files' ? job.reviewId : job.readyReviewId;
  useEffect(() => {
    if (!active || !id || !plan) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const next = await libraryData(
          { kind: 'organization_copy_job', job_id: id! },
          'organization_copy_job',
        );
        if (!live) return;
        if (
          next.id !== id ||
          next.planId !== plan ||
          next.reviewId !== stream ||
          next.sourceRootId !== job.sourceRootId ||
          next.action !== 'copy_files' ||
          next.totalFiles !== job.totalFiles
        )
          throw new Error('The continuation job belongs to another operation.');
        setCurrent(next);
        if (copyJobActive(next)) timer = setTimeout(() => void poll(), 250);
        else await onRefresh?.();
      } catch (error) {
        if (live) {
          setError(errorMessage(error));
          timer = setTimeout(() => void poll(), 1000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 100);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [active, id, plan, stream, job.sourceRootId, job.totalFiles, onRefresh]);
  async function create() {
    if (pending || active) return;
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    setApproval(null);
    setAcknowledged(false);
    setCurrent(null);
    setReview(null);
    try {
      if (retainedReview.current) {
        await library({
          kind: 'organization_copy_continuation_discard',
          review_id: retainedReview.current,
        });
        retainedReview.current = null;
      }
      if (epoch.current !== token) return;
      const value = await libraryData(
        {
          kind: 'organization_copy_continuation_review',
          job_id: job.id,
          selection_id: selection.id,
        },
        'organization_copy_continuation_review',
      );
      if (
        value.priorJobId !== job.id ||
        value.sourceRootId !== job.sourceRootId ||
        value.planId !== job.planId ||
        value.files.length !== job.totalFiles ||
        value.assessment.planId !== value.planId ||
        value.assessment.manifestId !== value.manifestId ||
        !value.sourcesRetained ||
        value.independentBackup ||
        value.files.some((row) => !row.file.targetPath.startsWith(`${value.outputFolder}/`))
      )
        throw new Error('The continuation review does not match this retained job.');
      if (epoch.current !== token) {
        void library({ kind: 'organization_copy_continuation_discard', review_id: value.id }).catch(
          () => {},
        );
        return;
      }
      retainedReview.current = value.id;
      setPage(0);
      setReview(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  async function approve() {
    if (!review || !acknowledged || pending || active) return;
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    setApproval(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_continuation_approve',
          selection_id: selection.id,
          input: {
            reviewId: review.id,
            digest: review.digest,
            action: 'copy_files',
            fidelityLimitsAcknowledged: true,
          },
        },
        'organization_copy_approval',
      );
      if (epoch.current !== token) return;
      if (value.reviewId !== review.id || value.action !== 'copy_files')
        throw new Error('Approval does not match this continuation review.');
      setApproval(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  async function start() {
    if (!review || !approval || pending || active) return;
    const token = ++epoch.current;
    const ticket = approval.id;
    setApproval(null);
    setPending(true);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_continuation_start',
          selection_id: selection.id,
          input: { reviewId: review.id, digest: review.digest, approvalId: ticket },
        },
        'organization_copy_job',
      );
      if (epoch.current !== token) return;
      if (
        value.id === job.id ||
        value.planId !== review.planId ||
        value.reviewId !== stream ||
        value.sourceRootId !== job.sourceRootId ||
        value.action !== 'copy_files' ||
        value.totalFiles !== review.files.length
      )
        throw new Error('The native continuation job does not match this review.');
      setCurrent(value);
      await onRefresh?.();
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <section aria-label="Reviewed copy continuation" aria-busy={pending}>
      <Button type="button" disabled={pending || active} onClick={() => void create()}>
        Review verification and continuation
      </Button>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {review && (
        <>
          <h4 ref={heading} tabIndex={-1}>
            Continuation review
          </h4>
          <p>
            <bdi>{review.destinationDisplayPath}</bdi> / <bdi>{review.outputFolder}</bdi>
          </p>
          <p>
            All retained bytes will be freshly compared with their sources. Matching stages may be
            continued; matching published files remain in place. Originals remain. Recorded phases
            below describe history; fresh verification is pending.
          </p>
          {review.recordedComplete && (
            <p>All files have recorded completion. This action freshly verifies them.</p>
          )}
          <p>
            Additional data allowance:{' '}
            {BigInt(review.assessment.capacity.roundedDataBytes).toLocaleString()} bytes. Estimated
            peak additional space:{' '}
            {BigInt(review.assessment.capacity.estimatedPeakAdditionalBytes).toLocaleString()}{' '}
            bytes. Observed available:{' '}
            {BigInt(review.assessment.capacity.observedAvailableBytes).toLocaleString()} bytes.
            Space is not reserved.
          </p>
          <p>
            The application journal also requires 128 MiB at admission, with its existing 32 MiB
            allowance credited on a shared volume. Runtime output and journal reserves remain in
            force.
          </p>
          <div className="organization-table">
            <table>
              <caption>Retained source and output paths</caption>
              <thead>
                <tr>
                  <th>Source</th>
                  <th>Output</th>
                  <th>Recorded phase</th>
                  <th>Retained stage bytes</th>
                </tr>
              </thead>
              <tbody>
                {review.files.slice(page * 64, (page + 1) * 64).map((row) => (
                  <tr key={row.file.id}>
                    <td>
                      <bdi>{row.file.sourcePath}</bdi>
                    </td>
                    <td>
                      <bdi>{row.file.targetPath}</bdi>
                    </td>
                    <td>{phases[row.recordedPhase]}</td>
                    <td>
                      {row.retainedStageBytes ??
                        (row.outputPresent ? 'Published output' : 'No stage')}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>
            Previous continuation files
          </Button>
          <Button
            type="button"
            disabled={(page + 1) * 64 >= review.files.length}
            onClick={() => setPage(page + 1)}
          >
            Next continuation files
          </Button>
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              disabled={pending || active}
              onChange={(event) => {
                setAcknowledged(event.target.checked);
                setApproval(null);
              }}
            />{' '}
            I understand that file bytes and selected companions are verified, while timestamps,
            permissions, extended attributes/resource forks, alternate streams and
            sparse/compression layout are not preserved or verified. Originals remain; this is not
            independent backup.
          </label>
          <Button
            type="button"
            disabled={pending || active || !acknowledged}
            onClick={() => void approve()}
          >
            Approve verification and continuation
          </Button>
          {approval && (
            <Button type="button" disabled={pending || active} onClick={() => void start()}>
              Start reviewed continuation
            </Button>
          )}
        </>
      )}
      {current && <CopyJobProgress job={current} {...(onRefresh ? { onRefresh } : {})} />}
      {active && <CopyIoControls />}
    </section>
  );
}
