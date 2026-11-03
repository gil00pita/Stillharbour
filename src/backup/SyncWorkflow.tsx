import { Button, Input, Select } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import type {
  ComparisonEntry,
  ComparisonRun,
  SyncAction,
  SyncApproval,
  SyncCapabilities,
  SyncJob,
  SyncReply,
  SyncRequest,
  SyncReview,
  SyncRetainedVersion,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { OldDestinationBackup } from './OldDestinationBackup';

type ReplyData<K extends SyncReply['kind']> = Extract<SyncReply, { kind: K }>['data'];
async function request<K extends SyncReply['kind']>(
  input: SyncRequest,
  kind: K,
): Promise<ReplyData<K>> {
  const reply = await libraryData({ kind: 'backup_sync', request: input }, 'backup_sync');
  if (reply.kind !== kind) throw new Error('Unexpected safe-copy response. Refresh job history.');
  return reply.data as ReplyData<K>;
}
const active = (job: SyncJob) =>
  ['queued', 'running', 'paused', 'cancel_requested'].includes(job.state);
const actionNames: Record<SyncAction, string> = {
  prepare_names: 'Prepare destination names',
  copy_files: 'Copy new files',
  resume_files: 'Continue verified copies',
  replace_retaining: 'Replace destination and retain previous version',
  restore_retained: 'Restore retained version to a new output',
};
function matchesJob(value: SyncJob, expected: SyncJob) {
  if (
    value.id !== expected.id ||
    value.runId !== expected.runId ||
    value.sourceRootId !== expected.sourceRootId
  )
    throw new Error(
      'The returned job belongs to another scope. Refresh history and inspect recovery.',
    );
}

export function SyncWorkflow({
  run,
  selected,
  onActivityChange,
}: {
  run: ComparisonRun | null;
  selected: ComparisonEntry | null;
  onActivityChange: (active: boolean) => void;
}) {
  const [capabilities, setCapabilities] = useState<SyncCapabilities | null>(null);
  const [capabilityError, setCapabilityError] = useState<string | null>(null);
  const [jobs, setJobs] = useState<SyncJob[]>([]);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyLoading, setHistoryLoading] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const [keep, setKeep] = useState<Set<string>>(() => new Set());
  const [review, setReview] = useState<SyncReview | null>(null);
  const [approval, setApproval] = useState<SyncApproval | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [requestBusy, setBusy] = useState(false);
  const [recoveryBusy, setRecoveryBusy] = useState(false);
  const busy = requestBusy || recoveryBusy;
  const [rate, setRate] = useState('10485760');
  const [versions, setVersions] = useState<Record<string, SyncRetainedVersion[]>>({});
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      onActivityChange(false);
    };
  }, [onActivityChange]);
  useEffect(() => {
    onActivityChange(busy || jobs.some(active));
  }, [busy, jobs, onActivityChange]);
  useEffect(() => {
    let retired = false;
    void request({ kind: 'capabilities' }, 'capabilities')
      .then((value) => {
        if (!retired) {
          setCapabilities(value);
          setRate(value.bytesPerSecond);
          setCapabilityError(null);
        }
      })
      .catch((cause: unknown) => {
        if (!retired) setCapabilityError(errorMessage(cause));
      });
    void request({ kind: 'list' }, 'jobs')
      .then((value) => {
        if (!Array.isArray(value)) throw new Error('Safe-copy history could not be read.');
        if (!retired) {
          setJobs(value);
          setHistoryError(null);
        }
      })
      .catch((cause: unknown) => {
        if (!retired) setHistoryError(errorMessage(cause));
      })
      .finally(() => {
        if (!retired) setHistoryLoading(false);
      });
    return () => {
      retired = true;
    };
  }, [refresh]);
  const running = jobs.find(active);
  useEffect(() => {
    if (!running) return;
    let retired = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await request({ kind: 'job', job_id: running.id }, 'job');
        if (retired) return;
        matchesJob(next, running);
        setJobs((current) => current.map((job) => (job.id === running.id ? next : job)));
        setHistoryError(null);
        if (active(next)) timer = setTimeout(() => void poll(), 750);
      } catch (cause) {
        if (!retired) {
          setHistoryError(errorMessage(cause));
          timer = setTimeout(() => void poll(), 2000);
        }
      }
    };
    timer = setTimeout(() => void poll(), 500);
    return () => {
      retired = true;
      clearTimeout(timer);
    };
  }, [running]);
  async function action(work: () => Promise<void>) {
    if (pending.current) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      if (alive.current) setError(errorMessage(cause));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  function showReview(value: SyncReview) {
    setApproval(null);
    setReview(value);
  }
  function saveJob(value: SyncJob) {
    setJobs((current) => [value, ...current.filter((job) => job.id !== value.id)]);
  }
  const canReview =
    run?.state === 'needs_review' && run.mode === 'verified' && Boolean(run.planDigest);
  const difference =
    selected &&
    ['different_content', 'destination_edited_since_last_run'].includes(selected.category);
  const validRate = /^\d{1,10}$/.test(rate) && BigInt(rate) >= 1024n && BigInt(rate) <= 1073741824n;
  return (
    <section
      aria-label="Safe-copy workflow"
      aria-busy={busy}
      className="backup-operation"
      id="safe-copy-workflow"
    >
      <h2>Safe copy / one-way sync</h2>
      <p>
        Add reviewed source-only files to the Stillharbour dataset. Source files and
        destination-only files are retained.
      </p>
      {!capabilities && !capabilityError && <p role="status">Checking transfer availability…</p>}
      {capabilities && !capabilities.executionAvailable && (
        <p role="status">{capabilities.reason}</p>
      )}
      {capabilityError && <p role="alert">{capabilityError}</p>}
      {difference && selected && (
        <div className="backup-decision">
          <p className="backup-path">Different destination: {selected.displayPath}</p>
          <label>
            <input
              type="checkbox"
              checked={keep.has(selected.id)}
              disabled={busy || Boolean(running) || !canReview}
              onChange={(event) => {
                const checked = event.target.checked;
                setKeep((current) => {
                  const next = new Set(current);
                  if (checked) next.add(selected.id);
                  else next.delete(selected.id);
                  return next;
                });
                setReview(null);
                setApproval(null);
                setError(null);
              }}
            />
            Keep this destination version; leave its differing source revision uncopied
          </label>
          <Button disabled aria-describedby="retained-replacement-gate">
            Replace destination and retain previous version
          </Button>
          <p id="retained-replacement-gate">
            Replacement requires a separate native review proving recovery of this exact existing
            destination version and its required metadata. Back up the old version below, then
            inspect that snapshot's replacement gate. Unverified independent recovery keeps
            replacement blocked.
          </p>
          {run && (
            <OldDestinationBackup
              key={`${run.id}:${selected.id}`}
              runId={run.id}
              entryId={selected.id}
              disabled={busy || Boolean(running) || !canReview}
              onActivityChange={setRecoveryBusy}
              onReplacementReview={(value) => {
                if (value.sourceRootId !== run.sourceRootId)
                  throw new Error('Retained replacement scope changed. Recompare.');
                showReview(value);
              }}
            />
          )}
        </div>
      )}
      <p>
        {keep.size} explicit keep-destination decisions. Decisions persist across pages of this
        comparison.
      </p>
      <Button
        disabled={busy || Boolean(running) || !canReview}
        onClick={() =>
          void action(async () => {
            if (!run) return;
            const value = await request(
              { kind: 'review_append', run_id: run.id, keep_destination: [...keep] },
              'review',
            );
            if (!alive.current) return;
            if (value.runId !== run.id || value.sourceRootId !== run.sourceRootId)
              throw new Error('The append review belongs to another comparison. Recompare.');
            showReview(value);
          })
        }
      >
        Review append plan
      </Button>
      {!canReview && (
        <p>
          A complete Verified comparison is required. Every difference needs an explicit decision.
        </p>
      )}
      {review && (
        <SyncReviewPanel
          key={`${review.id}:${review.digest}`}
          review={review}
          approved={Boolean(approval)}
          busy={busy}
          onDismiss={() => {
            setReview(null);
            setApproval(null);
          }}
          onApprove={(uncopied, metadata) =>
            void action(async () => {
              const value = await request(
                {
                  kind: 'approve',
                  review_id: review.id,
                  acknowledge_uncopied: uncopied,
                  acknowledge_metadata_limits: metadata,
                },
                'approval',
              );
              if (!alive.current) return;
              if (value.reviewId !== review.id || value.action !== review.action)
                throw new Error('Approval belongs to another action. Review the plan again.');
              setApproval(value);
            })
          }
          onStart={() =>
            void action(async () => {
              if (!approval || approval.reviewId !== review.id || approval.action !== review.action)
                return;
              const approvalId = approval.id;
              setApproval(null);
              setReview(null);
              let value: SyncJob;
              try {
                value = await request({ kind: 'start', approval_id: approvalId }, 'job');
              } catch (cause) {
                throw new Error(
                  `${errorMessage(cause)} Refresh safe-copy history and inspect any recorded job before retrying.`,
                  { cause },
                );
              }
              if (!alive.current) return;
              if (
                value.runId !== review.runId ||
                value.sourceRootId !== review.sourceRootId ||
                value.action !== review.action
              )
                throw new Error(
                  'Started job identity does not match this review. Inspect job history.',
                );
              saveJob(value);
              setReview(null);
              setApproval(null);
            })
          }
        />
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <details className="backup-io">
        <summary>Application I/O budget</summary>
        <p>
          {capabilities?.ioScope ??
            'The backend reports which application-owned reads and writes it can pace.'}{' '}
          This is a logical byte budget; it does not cap physical disk traffic or other
          applications.
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (validRate)
              void action(async () => {
                const value = await request(
                  { kind: 'set_limit', bytes_per_second: rate },
                  'capabilities',
                );
                if (alive.current) {
                  setCapabilities(value);
                  setRate(value.bytesPerSecond);
                }
              });
          }}
        >
          <label>
            Budget preset{' '}
            <Select
              disabled={busy}
              value={rate === '10485760' || rate === '41943040' ? rate : 'custom'}
              onChange={(event) => {
                if (event.target.value !== 'custom') setRate(event.target.value);
              }}
            >
              <option value="10485760">Quiet — 10 MiB/s</option>
              <option value="41943040">Balanced — 40 MiB/s</option>
              <option value="custom">Custom</option>
            </Select>
          </label>
          <label>
            Maximum issued bytes per second{' '}
            <Input
              inputMode="numeric"
              maxLength={10}
              value={rate}
              disabled={busy}
              onChange={(event) => setRate(event.target.value)}
            />
          </label>
          <Button type="submit" disabled={busy || !validRate}>
            Apply shared budget
          </Button>
        </form>
        <p>
          Changes affect comparison, copying and verification using this shared application budget.
          Allowed range: 1 KiB/s–1 GiB/s.
        </p>
      </details>
      <div className="backup-controls">
        <h3>Transfer history and recovery</h3>
        <Button
          disabled={busy}
          onClick={() => {
            setHistoryLoading(true);
            setRefresh((current) => current + 1);
          }}
        >
          Refresh safe-copy history
        </Button>
      </div>
      {historyError && (
        <p className="error" role="alert">
          {historyError}
        </p>
      )}
      {historyLoading && <p role="status">Loading safe-copy history…</p>}
      {!jobs.length && !historyError && !historyLoading && (
        <p>No safe-copy jobs are recorded in this catalog.</p>
      )}
      <ul className="backup-job-list">
        {jobs.map((job) => (
          <li key={job.id} className="backup-job">
            <h4>
              {actionNames[job.action]} · {job.state.replaceAll('_', ' ')}
            </h4>
            <p className="backup-path">
              {job.sourceDisplayPath} → {job.datasetDisplayPath}
            </p>
            <p role="status">{job.message}</p>
            <p>
              Phase: {job.phase.replaceAll('_', ' ')} · {job.verifiedFiles} of {job.totalFiles}{' '}
              files byte-verified · {job.copyBytes} planned bytes
            </p>
            <p>
              {job.uncopiedSourceRevisions} source revisions uncopied ·{' '}
              {job.retainedDestinationFiles} destination files retained · configured budget{' '}
              {job.bytesPerSecond} bytes/s
            </p>
            {job.uncopiedSourceRevisions > 0 && (
              <p>
                This job does not establish complete protection of the source. Its uncopied
                revisions remain at the source.
              </p>
            )}
            <small>Updated {job.updatedAt}</small>
            <div className="backup-controls">
              {job.action === 'replace_retaining' && (
                <Button
                  disabled={busy}
                  onClick={() =>
                    void action(async () => {
                      const value = await request({ kind: 'versions', job_id: job.id }, 'versions');
                      if (value.some((version) => version.jobId !== job.id))
                        throw new Error('Retained version identity changed. Refresh history.');
                      if (alive.current)
                        setVersions((current) => ({ ...current, [job.id]: value }));
                    })
                  }
                >
                  Inspect retained versions
                </Button>
              )}
              {(job.state === 'running' || job.state === 'paused') && (
                <Button
                  disabled={busy}
                  onClick={() =>
                    void action(async () => {
                      const value = await request(
                        { kind: job.state === 'paused' ? 'resume' : 'pause', job_id: job.id },
                        'job',
                      );
                      matchesJob(value, job);
                      if (alive.current) saveJob(value);
                    })
                  }
                >
                  {job.state === 'paused' ? 'Resume transfer' : 'Pause transfer'}
                </Button>
              )}
              {active(job) && (
                <Button
                  disabled={busy || job.state === 'cancel_requested'}
                  onClick={() =>
                    void action(async () => {
                      const value = await request({ kind: 'cancel', job_id: job.id }, 'job');
                      matchesJob(value, job);
                      if (alive.current) saveJob(value);
                    })
                  }
                >
                  Cancel transfer
                </Button>
              )}
              {job.state === 'needs_review' && job.readyReviewId && (
                <Button
                  disabled={busy || Boolean(running)}
                  onClick={() =>
                    void action(async () => {
                      const value = await request(
                        { kind: 'review', review_id: job.readyReviewId! },
                        'review',
                      );
                      if (
                        value.runId !== job.runId ||
                        value.sourceRootId !== job.sourceRootId ||
                        value.action !== 'copy_files'
                      )
                        throw new Error('Prepared copy review scope changed. Inspect recovery.');
                      if (alive.current) showReview(value);
                    })
                  }
                >
                  Review prepared copy
                </Button>
              )}
              {['needs_recovery', 'failed', 'canceled', 'completed_partial'].includes(
                job.state,
              ) && (
                <Button
                  disabled={busy || Boolean(running)}
                  onClick={() =>
                    void action(async () => {
                      const value = await request(
                        { kind: 'recovery_review', job_id: job.id },
                        'review',
                      );
                      if (value.runId !== job.runId || value.sourceRootId !== job.sourceRootId)
                        throw new Error('Recovery review scope changed. Refresh history.');
                      if (alive.current) showReview(value);
                    })
                  }
                >
                  Inspect recovery
                </Button>
              )}
            </div>
            {versions[job.id] && (
              <section aria-label="Retained destination versions">
                {!versions[job.id]?.length && (
                  <p>No retained versions were returned. This does not establish recoverability.</p>
                )}
                <ul>
                  {versions[job.id]?.map((version) => (
                    <li key={version.id}>
                      <p className="backup-path">{version.displayPath}</p>
                      <p>
                        {version.bytes} bytes · Full digest: {version.fullDigest}
                      </p>
                      <p>
                        {version.retained
                          ? 'Retained version recorded'
                          : 'Retention not established'}{' '}
                        · {version.reason}
                      </p>
                    </li>
                  ))}
                </ul>
                <Button
                  disabled={
                    busy ||
                    Boolean(running) ||
                    !versions[job.id]?.length ||
                    versions[job.id]?.some((version) => !version.restoreAvailable)
                  }
                  onClick={() =>
                    void action(async () => {
                      const value = await request(
                        { kind: 'review_restore_retained', job_id: job.id },
                        'review',
                      );
                      if (
                        value.runId !== job.runId ||
                        value.sourceRootId !== job.sourceRootId ||
                        value.action !== 'restore_retained'
                      )
                        throw new Error('Retained restore review scope changed. Inspect recovery.');
                      if (alive.current) showReview(value);
                    })
                  }
                >
                  Review retained restore to new output
                </Button>
                <p>
                  Restore requires a separate approval and writes into a new protected output.
                  Existing originals and retained history remain untouched.
                </p>
              </section>
            )}
          </li>
        ))}
      </ul>
      {running && (
        <p>
          Pause applies at safe I/O boundaries and uses the shared application budget. Cancellation
          retains sources and any staged/published objects for inspection.
        </p>
      )}
    </section>
  );
}

function SyncReviewPanel({
  review,
  approved,
  busy,
  onApprove,
  onStart,
  onDismiss,
}: {
  review: SyncReview;
  approved: boolean;
  busy: boolean;
  onApprove: (uncopied: boolean, metadata: boolean) => void;
  onStart: () => void;
  onDismiss: () => void;
}) {
  const [scopeReviewed, setScopeReviewed] = useState(false);
  const [uncopied, setUncopied] = useState(false);
  const [metadata, setMetadata] = useState(false);
  const [offset, setOffset] = useState(0);
  const items = review.items.slice(offset, offset + 200);
  return (
    <section className="backup-review" aria-label="Safe-copy action review">
      <h3>{actionNames[review.action]}</h3>
      <p className="backup-path">Source: {review.sourceDisplayPath}</p>
      <p className="backup-path">Dataset: {review.datasetDisplayPath}</p>
      <dl className="backup-review-totals">
        <div>
          <dt>Files proposed for copy</dt>
          <dd>{review.copyFiles}</dd>
        </div>
        <div>
          <dt>Content bytes</dt>
          <dd>{review.copyBytes}</dd>
        </div>
        <div>
          <dt>Estimated required space</dt>
          <dd>{review.requiredBytes} bytes</dd>
        </div>
        <div>
          <dt>Available space</dt>
          <dd>
            {review.availableBytes === null
              ? 'Unknown — backend review required'
              : `${review.availableBytes} bytes`}
          </dd>
        </div>
        <div>
          <dt>Destination files retained</dt>
          <dd>{review.retainedDestinationFiles}</dd>
        </div>
        <div>
          <dt>Uncopied source revisions</dt>
          <dd>{review.uncopiedSourceRevisions}</dd>
        </div>
      </dl>
      <p>{review.metadataWarning}</p>
      <p>
        This action includes all {review.items.length} plan entries. Display pages do not change the
        approved scope.
      </p>
      <div className="backup-table">
        <table>
          <caption>Exact safe-copy plan</caption>
          <thead>
            <tr>
              <th>Relative path</th>
              <th>Action</th>
              <th>Source bytes</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.entryId}>
                <td className="backup-path">{item.displayPath}</td>
                <td>{item.action.replaceAll('_', ' ')}</td>
                <td>{item.sourceBytes ?? 'Not applicable'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="backup-controls">
        <Button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - 200))}>
          Previous plan entries
        </Button>
        <span>
          Plan entries {items.length ? offset + 1 : 0}–{offset + items.length} of{' '}
          {review.items.length}
        </span>
        <Button
          disabled={offset + 200 >= review.items.length}
          onClick={() => setOffset(offset + 200)}
        >
          Next plan entries
        </Button>
      </div>
      {!review.executionAvailable && <p role="status">{review.reason}</p>}
      <fieldset disabled={busy || approved || !review.executionAvailable}>
        <legend>Confirm this exact action</legend>
        <label>
          <input
            type="checkbox"
            checked={scopeReviewed}
            onChange={(event) => setScopeReviewed(event.target.checked)}
          />
          I reviewed the source, destination and proposed action
        </label>
        {review.uncopiedSourceRevisions > 0 && (
          <label>
            <input
              type="checkbox"
              checked={uncopied}
              onChange={(event) => setUncopied(event.target.checked)}
            />
            I understand that {review.uncopiedSourceRevisions} source revisions will remain uncopied
          </label>
        )}
        <label>
          <input
            type="checkbox"
            checked={metadata}
            onChange={(event) => setMetadata(event.target.checked)}
          />
          I reviewed the metadata limits; this does not establish independent recovery
        </label>
      </fieldset>
      <p>
        Approval expires after {review.approvalLifetimeSeconds} seconds. Preparation, copying and
        recovery each require their own review.
      </p>
      <div className="backup-controls">
        {!approved && (
          <Button
            disabled={
              busy ||
              !review.executionAvailable ||
              !scopeReviewed ||
              !metadata ||
              (review.uncopiedSourceRevisions > 0 && !uncopied)
            }
            onClick={() => onApprove(uncopied, metadata)}
          >
            Approve this safe-copy action
          </Button>
        )}
        {approved && (
          <Button
            className="primary"
            disabled={busy || !review.executionAvailable}
            onClick={onStart}
          >
            Start {actionNames[review.action].toLowerCase()}
          </Button>
        )}
        <Button disabled={busy} onClick={onDismiss}>
          Close action review
        </Button>
      </div>
    </section>
  );
}
