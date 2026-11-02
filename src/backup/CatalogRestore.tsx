import { useEffect, useRef, useState } from 'react';
import type {
  CatalogRestoreApproval,
  CatalogRestorePrepared,
  CatalogRestoreReview,
  SnapshotJob,
} from '../bindings';
import { errorMessage, pickDirectory } from '../ipc';
import { snapshotRequest } from './snapshotIpc';
import { catalogSwitchStore } from './catalogSwitchState';

export function CatalogRestore({
  job,
  catalogId,
  disabled,
  onActivityChange,
}: {
  job: SnapshotJob;
  catalogId: string;
  disabled: boolean;
  onActivityChange: (jobId: string, active: boolean) => void;
}) {
  if (job.action !== 'restore') return null;
  if (
    job.state !== 'completed_verified' ||
    !job.snapshotId ||
    !job.included ||
    !job.structuralChecked ||
    !job.dataReadVerified ||
    !job.restoredCompared ||
    !job.sourceRetained
  ) {
    return (
      <p>
        Catalog switch review requires a completed whole-snapshot restore with full verification.
      </p>
    );
  }
  return (
    <CatalogRestoreWorkspace
      key={`${catalogId}:${job.id}:${job.snapshotId}`}
      job={job}
      catalogId={catalogId}
      disabled={disabled}
      onActivityChange={onActivityChange}
    />
  );
}

function CatalogRestoreWorkspace({
  job,
  catalogId,
  disabled,
  onActivityChange,
}: {
  job: SnapshotJob;
  catalogId: string;
  disabled: boolean;
  onActivityChange: (jobId: string, active: boolean) => void;
}) {
  const [review, setReview] = useState<CatalogRestoreReview | null>(null);
  const [approval, setApproval] = useState<CatalogRestoreApproval | null>(null);
  const [prepared, setPrepared] = useState<CatalogRestorePrepared | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState('');
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      onActivityChange(job.id, false);
    };
  }, [job.id, onActivityChange]);
  useEffect(() => {
    onActivityChange(job.id, busy || Boolean(prepared) || uncertain);
  }, [job.id, busy, prepared, uncertain, onActivityChange]);

  function validate(value: CatalogRestoreReview) {
    if (
      value.restoreJobId !== job.id ||
      value.snapshotId !== job.snapshotId ||
      value.catalogId !== catalogId ||
      !value.id ||
      !/^[a-fA-F0-9]{64}$/.test(value.digest) ||
      !value.currentRevision ||
      !value.restoredRevision ||
      new Set(value.roots.map((root) => root.rootId)).size !== value.roots.length ||
      value.roots.some((root) => !root.rootId || (root.confirmed && !root.confirmedDisplayPath))
    ) {
      throw new Error('Catalog review identity or root mapping changed. Request a fresh review.');
    }
  }
  async function action(message: string, operation: () => Promise<void>) {
    if (disabled || pending.current || prepared || uncertain) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    setProgress(message);
    try {
      await operation();
    } catch (cause) {
      if (alive.current) setError(errorMessage(cause));
    } finally {
      if (alive.current) {
        pending.current = false;
        setBusy(false);
        setProgress('');
      }
    }
  }
  const blocked = disabled || busy || Boolean(prepared) || uncertain;
  const ready = review?.approvalAvailable && review.roots.every((root) => root.confirmed);
  return (
    <section className="backup-review" aria-label="Restored catalog switch">
      <h5>Use this restored catalog</h5>
      <p>
        Review switching Stillharbour to this restored catalog. The current catalog and its newer
        edits must be preserved separately; edits are not merged.
      </p>
      {!prepared && !uncertain && (
        <button
          disabled={blocked}
          onClick={() =>
            void action('Inspecting current and restored catalogs…', async () => {
              setApproval(null);
              setAcknowledged(false);
              setReview(null);
              const value = await snapshotRequest(
                { kind: 'catalog_restore_review', restore_job_id: job.id },
                'catalog_restore_review',
              );
              if (!alive.current) return;
              validate(value);
              setReview(value);
            })
          }
        >
          {review ? 'Refresh catalog switch review' : 'Review catalog switch'}
        </button>
      )}
      {review && (
        <>
          <dl className="backup-evidence">
            <div>
              <dt>Current and restored catalog UUID</dt>
              <dd>
                <code>{review.catalogId}</code>
              </dd>
            </div>
            <div>
              <dt>Validated catalog schema</dt>
              <dd>{review.schema}</dd>
            </div>
            <div>
              <dt>Current catalog revision</dt>
              <dd>
                <code>{review.currentRevision}</code>
              </dd>
            </div>
            <div>
              <dt>Restored catalog revision</dt>
              <dd>
                <code>{review.restoredRevision}</code>
              </dd>
            </div>
          </dl>
          <p>
            Exact review digest: <code>{review.digest}</code>
          </p>
          <p>{review.warning}</p>
          <p>
            Each source root requires a fresh native selection matching its saved identity and
            volume. Displayed paths do not authorize access.
          </p>
          <ul aria-label="Restored catalog root mappings">
            {review.roots.map((root) => (
              <li key={root.rootId}>
                <strong>{root.label}</strong> · Root UUID <code>{root.rootId}</code>
                <p className="backup-path">Saved location: {root.savedDisplayPath}</p>
                <p className="backup-path">
                  {root.confirmed
                    ? `Native identity confirmed: ${root.confirmedDisplayPath}`
                    : 'Native identity not confirmed'}
                </p>
                <button
                  disabled={blocked}
                  onClick={() =>
                    void action(
                      'Choose the original source root in the native picker…',
                      async () => {
                        setApproval(null);
                        setAcknowledged(false);
                        const selection = await pickDirectory();
                        if (!alive.current || !selection) return;
                        const value = await snapshotRequest(
                          {
                            kind: 'catalog_restore_map',
                            review_id: review.id,
                            root_id: root.rootId,
                            selection_id: selection.id,
                          },
                          'catalog_restore_review',
                        );
                        if (!alive.current) return;
                        validate(value);
                        if (
                          JSON.stringify(value.roots.map((mapped) => mapped.rootId).sort()) !==
                            JSON.stringify(review.roots.map((mapped) => mapped.rootId).sort()) ||
                          value.restoredRevision !== review.restoredRevision ||
                          value.currentRevision !== review.currentRevision
                        ) {
                          throw new Error(
                            'Catalog restore scope changed during mapping. Request a fresh review.',
                          );
                        }
                        if (
                          !value.roots.some(
                            (mapped) => mapped.rootId === root.rootId && mapped.confirmed,
                          )
                        )
                          throw new Error('The selected root was not confirmed by the backend.');
                        setReview(value);
                      },
                    )
                  }
                >
                  Choose native location for {root.label}
                </button>
              </li>
            ))}
          </ul>
          {!ready && (
            <p role="status">
              Catalog approval is unavailable until every required native mapping and backend check
              is confirmed.
            </p>
          )}
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              disabled={blocked || Boolean(approval) || !ready}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I reviewed these exact catalog revisions and root mappings. Preserve my current catalog
            and newer edits separately; do not merge them.
          </label>
          {!approval ? (
            <button
              disabled={blocked || !ready || !acknowledged}
              onClick={() =>
                void action('Approving this exact catalog switch…', async () => {
                  const value = await snapshotRequest(
                    {
                      kind: 'catalog_restore_approve',
                      review_id: review.id,
                      digest: review.digest,
                    },
                    'catalog_restore_approval',
                  );
                  if (!alive.current) return;
                  if (!value.id || value.reviewId !== review.id || value.digest !== review.digest)
                    throw new Error(
                      'Catalog approval does not match this review. Request a fresh review.',
                    );
                  setApproval(value);
                })
              }
            >
              Approve catalog switch
            </button>
          ) : (
            <>
              <p>Approval recorded. Catalog preparation has not started.</p>
              <button
                disabled={blocked}
                onClick={() =>
                  void action(
                    'Preparing the catalog switch and preserving current edits…',
                    async () => {
                      const approved = approval;
                      setApproval(null);
                      let ticket: number | undefined;
                      try {
                        ticket = catalogSwitchStore.begin({
                          catalogId,
                          restoreJobId: job.id,
                          reviewDigest: review.digest,
                        });
                        const value = await snapshotRequest(
                          { kind: 'catalog_restore_prepare', approval_id: approved.id },
                          'catalog_restore_prepared',
                        );
                        if (
                          !value.transactionId ||
                          value.catalogId !== catalogId ||
                          !value.restartRequired ||
                          !value.currentCatalogRetained
                        )
                          throw new Error(
                            'Catalog preparation did not confirm its required preservation and restart state.',
                          );
                        catalogSwitchStore.prepared(ticket, value);
                        if (alive.current) setPrepared(value);
                      } catch (cause) {
                        if (ticket !== undefined)
                          catalogSwitchStore.unconfirmed(ticket, errorMessage(cause));
                        if (alive.current) setUncertain(true);
                        throw cause;
                      }
                    },
                  )
                }
              >
                Prepare catalog switch
              </button>
            </>
          )}
        </>
      )}
      {busy && <p role="status">{progress}</p>}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {uncertain && (
        <p role="status">
          Preparation was not confirmed. Close and reopen Stillharbour to reconcile the pending
          switch before another attempt. Existing catalogs are retained.
        </p>
      )}
      {prepared && (
        <div role="status">
          <p>{prepared.message}</p>
          <p>
            Pending restart. The current catalog is retained; this response does not confirm that
            the restored catalog is active.
          </p>
          <p>Close and reopen Stillharbour to validate and apply the prepared catalog switch.</p>
          <p>
            Catalog switch transaction: <code>{prepared.transactionId}</code>
          </p>
        </div>
      )}
    </section>
  );
}
