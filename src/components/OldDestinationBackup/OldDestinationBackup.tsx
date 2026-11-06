import { Field } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type {
  SnapshotApproval,
  SnapshotJob,
  SnapshotProfile,
  SnapshotReview,
  SyncReview,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { SnapshotReviewPanel } from '@/components/SnapshotWorkflow';
import { snapshotRequest } from '../../backup/snapshotIpc';
import { SnapshotRecoveryInspection } from '@/components/SnapshotRecoveryInspection';

const active = (job: SnapshotJob) => ['queued', 'running', 'cancel_requested'].includes(job.state);
export function OldDestinationBackup({
  runId,
  entryId,
  disabled,
  onActivityChange,
  onReplacementReview,
}: {
  runId: string;
  entryId: string;
  disabled: boolean;
  onActivityChange: (value: boolean) => void;
  onReplacementReview: (value: SyncReview) => void;
}) {
  const [profiles, setProfiles] = useState<SnapshotProfile[]>([]);
  const [profileId, setProfileId] = useState('');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [review, setReview] = useState<SnapshotReview | null>(null);
  const [approval, setApproval] = useState<SnapshotApproval | null>(null);
  const [job, setJob] = useState<SnapshotJob | null>(null);
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    void snapshotRequest({ kind: 'profiles' }, 'profiles')
      .then((value) => {
        if (alive.current) setProfiles(value);
      })
      .catch((cause: unknown) => {
        if (alive.current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (alive.current) setLoading(false);
      });
    return () => {
      alive.current = false;
      onActivityChange(false);
    };
  }, [onActivityChange]);
  const running = Boolean(job && active(job));
  useEffect(() => onActivityChange(busy || running), [busy, running, onActivityChange]);
  useEffect(() => {
    if (!job || !active(job)) return;
    let retired = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const value = await snapshotRequest({ kind: 'job', job_id: job.id }, 'job');
        if (retired) return;
        if (value.id !== job.id || value.profileId !== job.profileId || value.action !== job.action)
          throw new Error('Old destination backup job identity changed. Inspect snapshot history.');
        setJob(value);
        setError(null);
      } catch (cause) {
        if (!retired) {
          setError(errorMessage(cause));
          timer = setTimeout(() => void poll(), 2000);
        }
      }
    };
    timer = setTimeout(() => void poll(), 500);
    return () => {
      retired = true;
      clearTimeout(timer);
    };
  }, [job]);
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
  const locked = disabled || busy || running;
  return (
    <section className="backup-review" aria-label="Protect old destination versions">
      <h4>Back up old destination and companions</h4>
      <p>
        This protects the exact existing destination versions derived by the native comparison. It
        does not back up the newer source in their place and does not authorize replacement.
      </p>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Recovery snapshot profile </span>
          </Field.Label>
          <NativeSelect
            value={profileId}
            disabled={locked || loading}
            onChange={(event) => {
              setProfileId(event.target.value);
              setReview(null);
              setApproval(null);
              setJob(null);
            }}
          >
            <option value="">Choose a saved snapshot profile</option>
            {profiles.map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.label} · {profile.protocol}
              </option>
            ))}
          </NativeSelect>
        </label>
      </Field.Root>
      {!loading && !profiles.length && (
        <p>
          Create and authorize a repository profile in Versioned backup, then return to this
          difference.
        </p>
      )}
      <Button
        disabled={locked || !profileId}
        onClick={() =>
          void action(async () => {
            const reply = await libraryData(
              {
                kind: 'backup_sync',
                request: {
                  kind: 'review_recovery_backup',
                  run_id: runId,
                  entry_id: entryId,
                  profile_id: profileId,
                },
              },
              'backup_sync',
            );
            if (!alive.current) return;
            if (
              reply.kind !== 'recovery_backup' ||
              reply.data.profileId !== profileId ||
              reply.data.action !== 'backup'
            )
              throw new Error(
                'Old destination backup review identity changed. Recompare before approval.',
              );
            setReview(reply.data);
            setApproval(null);
          })
        }
      >
        Review old destination recovery backup
      </Button>
      {review && (
        <SnapshotReviewPanel
          key={`${review.id}:${review.digest}`}
          review={review}
          roots={review.rootIds.map((id) => ({ id, label: 'Exact reviewed old destination' }))}
          busy={locked}
          approved={Boolean(approval)}
          onClose={() => {
            setReview(null);
            setApproval(null);
          }}
          onApprove={() =>
            void action(async () => {
              const value = await snapshotRequest(
                { kind: 'backup_approve', review_id: review.id, digest: review.digest },
                'approval',
              );
              if (!alive.current) return;
              if (value.reviewId !== review.id || value.digest !== review.digest)
                throw new Error('Old destination backup approval does not match this review.');
              setApproval(value);
            })
          }
          onStart={() =>
            void action(async () => {
              if (!approval || approval.reviewId !== review.id || approval.digest !== review.digest)
                return;
              const approved = approval;
              setApproval(null);
              setReview(null);
              let value: SnapshotJob;
              try {
                value = await snapshotRequest(
                  { kind: 'backup_start', review_id: review.id, approval_id: approved.id },
                  'job',
                );
              } catch (cause) {
                throw new Error(
                  `${errorMessage(cause)} Inspect snapshot history before another request; the start result may be uncertain.`,
                  { cause },
                );
              }
              if (!alive.current) return;
              if (value.profileId !== profileId || value.action !== 'backup')
                throw new Error(
                  'Started old destination backup has another profile or action. Inspect history.',
                );
              setJob(value);
            })
          }
        />
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {job && (
        <div aria-label="Old destination backup job">
          <p role="status">
            {job.state.replaceAll('_', ' ')} · {job.message}
          </p>
          <p>
            Included: {job.included ? 'Yes' : 'Not established'} · Structural check:{' '}
            {job.structuralChecked ? 'Yes' : 'Not established'} · Stored data read:{' '}
            {job.dataReadVerified ? 'Yes' : 'Not established'} · Restored bytes compared:{' '}
            {job.restoredCompared ? 'Yes' : 'Not established'}
          </p>
          {job.snapshotId && (
            <p>
              Recorded recovery snapshot: <code>{job.snapshotId}</code>
            </p>
          )}
          <p>
            Essential metadata and independent recovery evidence are separate gates. This snapshot
            does not automatically enable replacement.
          </p>
          {job.action === 'backup' && job.state === 'needs_recovery' && (
            <SnapshotRecoveryInspection
              key={job.id}
              job={job}
              disabled={busy || running}
              onInspect={(snapshotId) =>
                void action(async () => {
                  setReview(null);
                  setApproval(null);
                  const value = await snapshotRequest(
                    {
                      kind: 'reconcile',
                      profile_id: profileId,
                      job_id: job.id,
                      snapshot_id: snapshotId,
                    },
                    'job',
                  );
                  if (
                    value.id === job.id ||
                    value.profileId !== profileId ||
                    value.action !== 'reconcile'
                  )
                    throw new Error(
                      'Reconciled backup identity changed. Inspect snapshot history.',
                    );
                  if (alive.current) setJob(value);
                })
              }
            />
          )}
          {job.snapshotId && job.state === 'completed_verified' && (
            <Button
              disabled={locked}
              onClick={() =>
                void action(async () => {
                  const reply = await libraryData(
                    {
                      kind: 'backup_sync',
                      request: {
                        kind: 'review_replacement',
                        run_id: runId,
                        entry_id: entryId,
                        recovery_evidence_id: job.snapshotId!,
                      },
                    },
                    'backup_sync',
                  );
                  if (!alive.current) return;
                  if (
                    reply.kind !== 'review' ||
                    reply.data.runId !== runId ||
                    reply.data.action !== 'replace_retaining'
                  )
                    throw new Error('Retained replacement review identity changed. Recompare.');
                  onReplacementReview(reply.data);
                })
              }
            >
              Inspect retained replacement gate for this snapshot
            </Button>
          )}
          {active(job) && (
            <Button
              disabled={busy}
              onClick={() =>
                void action(async () => {
                  const value = await snapshotRequest({ kind: 'cancel', job_id: job.id }, 'job');
                  if (value.id !== job.id || value.profileId !== job.profileId)
                    throw new Error('Canceled backup job identity changed. Inspect history.');
                  if (alive.current) setJob(value);
                })
              }
            >
              Cancel old destination backup
            </Button>
          )}
        </div>
      )}
    </section>
  );
}
