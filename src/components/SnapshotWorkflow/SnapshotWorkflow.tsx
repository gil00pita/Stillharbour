import { Field, Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { UtilityWelcome } from '@/components/UtilityWelcome';
import { UtilityHistory } from '@/components/UtilityHistory';
import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  LibraryStatus,
  SnapshotApproval,
  SnapshotBrowse,
  SnapshotCapabilities,
  SnapshotJob,
  SnapshotProfile,
  SnapshotRecord,
  SnapshotReview,
  SnapshotSchedule,
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';
import { SnapshotProfileForm } from '@/components/SnapshotProfileForm';
import { snapshotRequest } from '../../backup/snapshotIpc';
import { SnapshotRecoveryInspection } from '@/components/SnapshotRecoveryInspection';
import { CatalogRestore, RetainedCatalogRestore } from '@/components/CatalogRestore';

const active = (job: SnapshotJob) => ['queued', 'running', 'cancel_requested'].includes(job.state);
function validJob(value: SnapshotJob, profileId: string, expected?: SnapshotJob) {
  if (
    value.profileId !== profileId ||
    (expected && (value.id !== expected.id || value.action !== expected.action))
  )
    throw new Error('Snapshot job identity changed. Refresh this profile and inspect history.');
}
function Evidence({
  value,
}: {
  value: Pick<
    SnapshotRecord,
    'included' | 'structuralChecked' | 'dataReadVerified' | 'restoredCompared'
  >;
}) {
  return (
    <dl className="backup-evidence">
      <Box>
        <dt>Included in snapshot</dt>
        <dd>{value.included ? 'Recorded' : 'Not established'}</dd>
      </Box>
      <Box>
        <dt>Repository structure checked</dt>
        <dd>{value.structuralChecked ? 'Recorded' : 'Not established'}</dd>
      </Box>
      <Box>
        <dt>Stored data read and verified</dt>
        <dd>{value.dataReadVerified ? 'Recorded' : 'Not established'}</dd>
      </Box>
      <Box>
        <dt>Restored and byte-compared</dt>
        <dd>{value.restoredCompared ? 'Recorded' : 'Not established'}</dd>
      </Box>
    </dl>
  );
}

export function SnapshotWorkflow({
  status,
  catalogId,
  onActivityChange,
}: {
  status: LibraryStatus;
  catalogId?: string | null;
  onActivityChange: (active: boolean) => void;
}) {
  const [capabilities, setCapabilities] = useState<SnapshotCapabilities | null>(null);
  const [capabilityError, setCapabilityError] = useState<string | null>(null);
  const [catalogReviewEpoch, setCatalogReviewEpoch] = useState(0);
  const [profiles, setProfiles] = useState<SnapshotProfile[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [showSetup, setShowSetup] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const refreshProfileViews = useCallback(() => {
    setCapabilities(null);
    setRefresh((value) => value + 1);
  }, []);
  const [creationBusy, setCreationBusy] = useState(false);
  const [profileBusy, setProfileBusy] = useState(false);
  const [retainedActivities, setRetainedActivities] = useState<Set<string>>(() => new Set());
  const onRetainedActivity = useCallback((scopeId: string, active: boolean) => {
    setRetainedActivities((current) => {
      if (current.has(scopeId) === active) return current;
      const next = new Set(current);
      if (active) next.add(scopeId);
      else next.delete(scopeId);
      return next;
    });
  }, []);
  const locked = creationBusy || profileBusy || retainedActivities.size > 0;
  useEffect(() => {
    onActivityChange(locked);
  }, [locked, onActivityChange]);
  useEffect(() => {
    let retired = false;
    void Promise.allSettled([
      snapshotRequest({ kind: 'capabilities' }, 'capabilities')
        .then((caps) => {
          if (!retired) {
            setCapabilities(caps);
            setCapabilityError(null);
          }
        })
        .catch((cause: unknown) => {
          if (!retired) {
            setCapabilities(null);
            setCapabilityError(errorMessage(cause));
          }
        }),
      snapshotRequest({ kind: 'profiles' }, 'profiles')
        .then((saved) => {
          if (!Array.isArray(saved)) throw new Error('Snapshot profiles could not be read.');
          if (!retired) {
            setProfiles(saved);
            setError(null);
          }
        })
        .catch((cause: unknown) => {
          if (!retired) setError(errorMessage(cause));
        })
        .finally(() => {
          if (!retired) setLoading(false);
        }),
    ]);
    return () => {
      retired = true;
    };
  }, [refresh]);
  const selected = profiles.find((profile) => profile.id === selectedId);
  const sourceBinding = selected?.rootIds.map((id) => {
    const root = status.roots.find((root) => root.id === id);
    return [id, root?.displayPath, root?.availability];
  });
  return (
    <Box className="backup-workspace">
      {!loading && !profiles.length && (
        <UtilityWelcome
          title="Keep earlier versions within reach."
          description="Create a versioned backup profile to review its scope and destination. Backups and restores always need a separate review and approval."
        />
      )}
      <Heading as="h2" unstyled>
        Encrypted versioned backup
      </Heading>
      <Text>
        Snapshots retain earlier versions in a restic repository. They need the repository recovery
        password and a verified restore to establish recoverability.
      </Text>
      <Text>
        This is separate from a readable safe copy. Removing a source does not automatically delete
        earlier snapshots.
      </Text>
      <chakra.section className="backup-recovery" aria-label="Latest retained catalog">
        <Heading as="h3" unstyled>
          Catalog saved before the latest switch
        </Heading>
        <Text>
          Review the local catalog saved before your latest restore. Stillharbour checks that it is
          available and saves your current edits before preparing another switch.
        </Text>
        {capabilities?.latestRetainedCatalogTransactionId && catalogId ? (
          <RetainedCatalogRestore
            key={catalogReviewEpoch}
            transactionId={capabilities.latestRetainedCatalogTransactionId}
            catalogId={catalogId}
            disabled={locked}
            onActivityChange={onRetainedActivity}
          />
        ) : (
          <Text role="status">
            {!capabilities
              ? 'Local retained-catalog availability has not been established.'
              : !capabilities.latestRetainedCatalogTransactionId
                ? 'No eligible local retained-catalog source is available in this session.'
                : 'Current catalog identity is unavailable; a retained-catalog review cannot start.'}
          </Text>
        )}
      </chakra.section>
      {capabilityError && (
        <Text className="error" role="alert">
          {capabilityError}
        </Text>
      )}
      {loading && <Text role="status">Loading snapshot profiles…</Text>}
      {error && (
        <Text className="error" role="alert">
          {error}
        </Text>
      )}
      {capabilities?.unavailableReason && (
        <Text role="status">{capabilities.unavailableReason}</Text>
      )}
      <Box className="backup-controls">
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Snapshot profile</span>
            </Field.Label>
            <NativeSelect
              value={selectedId}
              disabled={locked || loading}
              onChange={(event) => {
                setSelectedId(event.target.value);
                setShowSetup(false);
              }}
            >
              <option value="">Choose a snapshot profile</option>
              {profiles.map((profile) => (
                <option value={profile.id} key={profile.id}>
                  {profile.label} · {profile.protocol}
                </option>
              ))}
            </NativeSelect>
          </label>
        </Field.Root>
        <Button
          disabled={locked || loading}
          onClick={() => {
            setSelectedId('');
            setShowSetup(true);
          }}
        >
          New snapshot profile
        </Button>
        <Button
          disabled={locked}
          onClick={() => {
            setLoading(true);
            refreshProfileViews();
          }}
        >
          Refresh snapshot profiles
        </Button>
      </Box>
      {!loading && !profiles.length && !error && (
        <Text>
          No encrypted snapshot profiles are recorded. Create one using the current base library.
        </Text>
      )}
      {selectedId && !selected && !loading && (
        <Text role="alert">
          This profile is unavailable. Refresh profiles; a missing profile is not an empty
          repository.
        </Text>
      )}
      {showSetup && (
        <SnapshotProfileForm
          key={JSON.stringify(
            status.roots.map((root) => [root.id, root.isBase, root.displayPath, root.availability]),
          )}
          status={status}
          capabilities={capabilities}
          disabled={profileBusy || retainedActivities.size > 0}
          onActivityChange={setCreationBusy}
          onClose={() => setShowSetup(false)}
          onSaved={(value) => {
            setProfiles((current) => [
              value,
              ...current.filter((profile) => profile.id !== value.id),
            ]);
            setSelectedId(value.id);
            setShowSetup(false);
            setCatalogReviewEpoch((current) => current + 1);
          }}
        />
      )}
      {selected && (
        <SnapshotProfileWorkspace
          key={JSON.stringify([selected.id, sourceBinding])}
          profile={selected}
          status={status}
          capabilities={capabilities}
          disabled={creationBusy || retainedActivities.size > 0}
          onActivityChange={setProfileBusy}
          onRefreshProfiles={refreshProfileViews}
          onProfileUpdated={(value) => {
            setProfiles((current) =>
              current.some((profile) => profile.id === value.id)
                ? current.map((profile) => (profile.id === value.id ? value : profile))
                : [value, ...current],
            );
            setSelectedId(value.id);
            setCatalogReviewEpoch((current) => current + 1);
          }}
        />
      )}
      {capabilities && (
        <chakra.aside className="backup-helper-limits">
          <Heading as="h3" unstyled>
            Transfer and schedule limits
          </Heading>
          <Text>{capabilities.helperLimits}</Text>
          <Text>{capabilities.scheduleLimits}</Text>
          <Text>
            Upload progress, a structural check and a restored-byte comparison are separate evidence
            levels. None alone proves an independent failure domain.
          </Text>
        </chakra.aside>
      )}
    </Box>
  );
}

function SnapshotProfileWorkspace({
  profile,
  status,
  capabilities,
  disabled,
  onActivityChange,
  onProfileUpdated,
  onRefreshProfiles,
}: {
  profile: SnapshotProfile;
  status: LibraryStatus;
  capabilities: SnapshotCapabilities | null;
  disabled: boolean;
  onActivityChange: (active: boolean) => void;
  onProfileUpdated: (value: SnapshotProfile) => void;
  onRefreshProfiles: () => void;
}) {
  const [snapshots, setSnapshots] = useState<SnapshotRecord[]>([]);
  const [versionId, setVersionId] = useState('');
  const [jobs, setJobs] = useState<SnapshotJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [checkingHistory, setCheckingHistory] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [localBusy, setBusy] = useState(false);
  const [catalogActivities, setCatalogActivities] = useState<Set<string>>(() => new Set());
  const onCatalogActivityChange = useCallback((jobId: string, active: boolean) => {
    setCatalogActivities((current) => {
      if (current.has(jobId) === active) return current;
      const next = new Set(current);
      if (active) next.add(jobId);
      else next.delete(jobId);
      return next;
    });
  }, []);
  const busy = disabled || localBusy || catalogActivities.size > 0;
  const [selectedSnapshot, setSelectedSnapshot] = useState<SnapshotRecord | null>(null);
  const [browse, setBrowse] = useState<SnapshotBrowse | null>(null);
  const [browseOffset, setBrowseOffset] = useState(0);
  const [browseLoading, setBrowseLoading] = useState(false);
  const [browseError, setBrowseError] = useState<string | null>(null);
  const [review, setReview] = useState<SnapshotReview | null>(null);
  const [approval, setApproval] = useState<SnapshotApproval | null>(null);
  const [schedule, setSchedule] = useState<SnapshotSchedule | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      onActivityChange(false);
    };
  }, [onActivityChange]);
  const running = jobs.find(active);
  useEffect(() => {
    onActivityChange(localBusy || catalogActivities.size > 0 || Boolean(running));
  }, [localBusy, catalogActivities.size, running, onActivityChange]);
  useEffect(() => {
    let retired = false;
    void Promise.allSettled([
      snapshotRequest({ kind: 'jobs', profile_id: profile.id }, 'jobs'),
      snapshotRequest({ kind: 'snapshots', profile_id: profile.id }, 'snapshots'),
      snapshotRequest({ kind: 'schedule_get', profile_id: profile.id }, 'schedule'),
    ])
      .then(([jobResult, historyResult, policyResult]) => {
        if (retired) return;
        const errors: string[] = [];
        if (jobResult.status === 'fulfilled') {
          const values = jobResult.value;
          if (!Array.isArray(values) || values.some((job) => job.profileId !== profile.id))
            errors.push('Snapshot job identity changed. Refresh the profile.');
          else setJobs(values);
        } else errors.push(errorMessage(jobResult.reason));
        if (historyResult.status === 'fulfilled') {
          const values = historyResult.value;
          if (
            !Array.isArray(values) ||
            values.some((snapshot) => snapshot.profileId !== profile.id)
          )
            errors.push('Snapshot history identity changed. Refresh the profile.');
          else setSnapshots(values);
        } else errors.push(errorMessage(historyResult.reason));
        if (policyResult.status === 'fulfilled') {
          const value = policyResult.value;
          if (value && value.profileId !== profile.id)
            errors.push('Stored schedule identity changed. Refresh the profile.');
          else setSchedule(value);
        } else errors.push(errorMessage(policyResult.reason));
        setHistoryError(errors.length ? errors.join(' ') : null);
      })
      .catch((cause: unknown) => {
        if (!retired) setHistoryError(errorMessage(cause));
      })
      .finally(() => {
        if (!retired) {
          setLoading(false);
          setCheckingHistory(false);
        }
      });
    return () => {
      retired = true;
    };
  }, [profile.id, refresh]);
  useEffect(() => {
    if (!schedule?.enabled || checkingHistory || busy || running) return;
    const timer = setTimeout(() => {
      setCheckingHistory(true);
      setRefresh((value) => value + 1);
    }, 5000);
    return () => clearTimeout(timer);
  }, [schedule?.enabled, checkingHistory, busy, running]);
  useEffect(() => {
    if (!running) return;
    let retired = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const value = await snapshotRequest({ kind: 'job', job_id: running.id }, 'job');
        if (retired) return;
        validJob(value, profile.id, running);
        setJobs((current) => current.map((job) => (job.id === value.id ? value : job)));
        setHistoryError(null);
        if (active(value)) timer = setTimeout(() => void poll(), 750);
        else {
          setLoading(true);
          setCheckingHistory(true);
          setRefresh((current) => current + 1);
          onRefreshProfiles();
        }
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
  }, [running, profile.id, onRefreshProfiles]);
  const snapshotId = selectedSnapshot?.id;
  useEffect(() => {
    if (!snapshotId) return;
    let retired = false;
    void snapshotRequest(
      { kind: 'browse', profile_id: profile.id, snapshot_id: snapshotId, offset: browseOffset },
      'browse',
    )
      .then((value) => {
        if (retired) return;
        if (value.profileId !== profile.id || value.snapshotId !== snapshotId)
          throw new Error('Snapshot browser identity changed. Select the snapshot again.');
        setBrowse(value);
        setBrowseError(null);
      })
      .catch((cause: unknown) => {
        if (!retired) setBrowseError(errorMessage(cause));
      })
      .finally(() => {
        if (!retired) setBrowseLoading(false);
      });
    return () => {
      retired = true;
    };
  }, [profile.id, snapshotId, browseOffset]);
  async function action(work: () => Promise<void>) {
    if (disabled || pending.current || catalogActivities.size > 0) return;
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
  function showReview(
    value: SnapshotReview,
    expectedAction: 'backup' | 'restore',
    expectedRootIds: readonly string[] = profile.rootIds,
  ) {
    if (
      value.profileId !== profile.id ||
      value.action !== expectedAction ||
      JSON.stringify([...value.rootIds].sort()) !== JSON.stringify([...expectedRootIds].sort())
    )
      throw new Error(
        'The snapshot action review belongs to another profile or action. Refresh and review again.',
      );
    setApproval(null);
    setReview(value);
  }
  function saveJob(value: SnapshotJob) {
    validJob(value, profile.id);
    setJobs((current) => [value, ...current.filter((job) => job.id !== value.id)]);
    if (!active(value)) onRefreshProfiles();
  }
  async function unlock(purpose: 'repository' | 'transport') {
    const reply = await libraryData(
      { kind: 'backup_credential_unlock', profile_id: profile.id, purpose },
      'backup_snapshots',
    );
    if (!alive.current) return;
    if (reply.kind !== 'profile')
      throw new Error('Unexpected credential unlock response. Refresh profiles.');
    const value = reply.data;
    if (
      value.id === profile.id ||
      value.catalogId !== profile.catalogId ||
      value.protocol !== profile.protocol ||
      value.destinationDisplay !== profile.destinationDisplay ||
      (profile.repositoryId !== null && value.repositoryId !== profile.repositoryId) ||
      JSON.stringify([...value.rootIds].sort()) !== JSON.stringify([...profile.rootIds].sort())
    )
      throw new Error(
        'Unlocked profile does not preserve the reviewed repository and source scope. Reopen profiles.',
      );
    setReview(null);
    setApproval(null);
    onProfileUpdated(value);
  }
  return (
    <chakra.section
      aria-label="Snapshot profile workspace"
      aria-busy={busy}
      className="backup-operation"
    >
      <Heading as="h3" unstyled>
        {profile.label}
      </Heading>
      <Box className="backup-scope">
        <chakra.section aria-label="Snapshot source roots">
          <Heading as="h4" unstyled>
            Bound source roots
          </Heading>
          <List.Root unstyled>
            {profile.rootIds.map((id) => {
              const root = status.roots.find((root) => root.id === id);
              return (
                <List.Item unstyled key={id}>
                  <strong>{root?.label ?? 'Unavailable root'}</strong> · Last observed access:{' '}
                  {root?.availability ?? 'not registered'}
                  <Text className="backup-path">{root?.displayPath ?? id}</Text>
                </List.Item>
              );
            })}
          </List.Root>
          <Text>
            This profile keeps these root identities when the base library changes. Offline roots
            remain part of the expected scope.
          </Text>
        </chakra.section>
        <chakra.section aria-label="Snapshot repository">
          <Heading as="h4" unstyled>
            Encrypted repository
          </Heading>
          <Text className="backup-path">{profile.destinationDisplay}</Text>
          <Text>
            {profile.protocol} · {profile.status.replaceAll('_', ' ')}
          </Text>
          <Text className="backup-path">
            Repository identity: {profile.repositoryId ?? 'Not established yet'}
          </Text>
          {profile.hostFingerprint && (
            <Text className="backup-path">
              Configured host fingerprint: {profile.hostFingerprint}
            </Text>
          )}
          <Text>
            Credentials:{' '}
            {profile.credentialsPresent
              ? 'Present in this device’s vault'
              : 'Unavailable or locked — unlock before starting'}
          </Text>
          {profile.insecureTransport && (
            <Text>Plain FTP credentials and connection metadata are unencrypted.</Text>
          )}
        </chakra.section>
      </Box>
      <Box className="backup-controls">
        <Button
          disabled={busy || Boolean(running)}
          onClick={() =>
            void action(async () => {
              const selection = profile.protocol === 'local' ? await pickDirectory() : null;
              if (profile.protocol === 'local' && (!selection || !alive.current)) return;
              if (selection && !selection.identityAvailable)
                throw new Error('The selected repository parent cannot supply a native identity.');
              const value = await snapshotRequest(
                {
                  kind: 'profile_open',
                  profile_id: profile.id,
                  selection_id: selection?.id ?? null,
                },
                'profile',
              );
              if (value.id !== profile.id || value.catalogId !== profile.catalogId)
                throw new Error('Profile identity changed. Reopen snapshot profiles.');
              if (alive.current) onProfileUpdated(value);
            })
          }
        >
          {profile.protocol === 'local'
            ? 'Reauthorize repository parent…'
            : 'Refresh profile authorization'}
        </Button>
        <Button
          disabled={busy || Boolean(running)}
          onClick={() =>
            void action(async () => {
              const value = await snapshotRequest(
                { kind: 'backup_review', profile_id: profile.id },
                'review',
              );
              if (alive.current) showReview(value, 'backup');
            })
          }
        >
          Review backup scope
        </Button>
        <Button
          disabled={busy || checkingHistory}
          onClick={() => {
            setLoading(true);
            setCheckingHistory(true);
            setRefresh((current) => current + 1);
            onRefreshProfiles();
          }}
        >
          Refresh snapshot history
        </Button>
      </Box>
      {profile.protocol === 'local' && (
        <Text>
          Reauthorization requires a fresh native selection of this profile's repository parent. A
          different storage identity is not silently adopted.
        </Text>
      )}
      <details className="backup-key-recovery">
        <summary>Repository password recovery</summary>
        <Text>
          Encrypted snapshots require the repository password. Keep an independent recovery copy;
          losing the secret can make the backup unreadable.
        </Text>
        <Text>
          Export writes an unencrypted recovery key into a new file selected through the native
          dialog. Store that file separately and protect it from disclosure.
        </Text>
        <Text>
          Unlock uses a native secret dialog and creates a new profile credential binding. Previous
          credentials and repository history remain retained. Enter the existing repository
          password; choosing another password does not recover encrypted data.
        </Text>
        <Box className="backup-controls">
          <Button
            disabled={busy || Boolean(running) || !capabilities?.credentialsAvailable}
            onClick={() => void action(() => unlock('repository'))}
          >
            Unlock repository password…
          </Button>
          {profile.protocol !== 'local' && (
            <Button
              disabled={busy || Boolean(running) || !capabilities?.credentialsAvailable}
              onClick={() => void action(() => unlock('transport'))}
            >
              Unlock remote credentials…
            </Button>
          )}
        </Box>
        <Button
          disabled={busy || !capabilities?.credentialsAvailable}
          onClick={() =>
            void action(async () => {
              const value = await libraryData(
                { kind: 'backup_credential_export', profile_id: profile.id },
                'backup_credential_exported',
              );
              if (alive.current) setExportMessage(value);
            })
          }
        >
          Export repository recovery key…
        </Button>
        {exportMessage && <Text role="status">{exportMessage}</Text>}
      </details>
      {!profile.credentialsPresent && (
        <Text role="status">
          One or more required credentials are unavailable or locked on this device. A successful
          old upload cannot substitute for the repository recovery password.
        </Text>
      )}
      {review && (
        <SnapshotReviewPanel
          key={`action:${review.id}:${review.digest}`}
          review={review}
          roots={status.roots}
          busy={busy}
          approved={Boolean(approval)}
          onClose={() => {
            setReview(null);
            setApproval(null);
          }}
          onApprove={() =>
            void action(async () => {
              const value = await snapshotRequest(
                {
                  kind: review.action === 'backup' ? 'backup_approve' : 'restore_approve',
                  review_id: review.id,
                  digest: review.digest,
                },
                'approval',
              );
              if (!alive.current) return;
              if (value.reviewId !== review.id || value.digest !== review.digest)
                throw new Error('Approval does not match this snapshot review. Review again.');
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
                  {
                    kind: review.action === 'backup' ? 'backup_start' : 'restore_start',
                    review_id: review.id,
                    approval_id: approved.id,
                  },
                  'job',
                );
              } catch (cause) {
                throw new Error(
                  `${errorMessage(cause)} Refresh snapshot history before requesting another action; the start result may be uncertain.`,
                  { cause },
                );
              }
              validJob(value, profile.id);
              if (value.action !== review.action)
                throw new Error('Started snapshot job action changed. Inspect history.');
              if (alive.current) saveJob(value);
            })
          }
        />
      )}
      {error && (
        <Text className="error" role="alert">
          {error}
        </Text>
      )}
      {historyError && (
        <Text className="error" role="alert">
          {historyError}
        </Text>
      )}
      {loading && <Text role="status">Loading snapshots and job history…</Text>}
      {!loading && !snapshots.length && !historyError && (
        <Text>
          No application snapshots are recorded for this profile. This does not prove that an
          unavailable repository is empty.
        </Text>
      )}
      <Box className="backup-controls">
        <Heading as="h4" unstyled>
          Snapshot history
        </Heading>
        <UtilityHistory
          versions={snapshots.map((snapshot) => ({
            id: snapshot.id,
            date: snapshot.time,
            label: `${snapshot.files} files · ${snapshot.bytes} content bytes`,
            state: snapshot.included ? 'included' : 'unverified',
          }))}
          selectedId={
            snapshots.find((snapshot) => snapshot.id === versionId)?.id ?? snapshots[0]?.id
          }
          disabled={busy}
          onSelect={(id) => {
            setVersionId(id);
            setSelectedSnapshot(null);
            setBrowse(null);
            setReview(null);
            setApproval(null);
          }}
        />
      </Box>
      <List.Root unstyled className="backup-job-list">
        {snapshots
          .filter(
            (snapshot) =>
              snapshot.id ===
              (snapshots.find((value) => value.id === versionId)?.id ?? snapshots[0]?.id),
          )
          .map((snapshot) => (
            <List.Item unstyled className="backup-job" key={snapshot.id}>
              <Heading as="h5" unstyled>
                {snapshot.time} · {snapshot.files} files · {snapshot.bytes} content bytes
              </Heading>
              <Text className="backup-path">Snapshot {snapshot.id}</Text>
              <Evidence value={snapshot} />
              <Text>
                Catalog included: {snapshot.catalogIncluded ? 'Yes' : 'Not established'} · Source
                retained: {snapshot.sourceRetained ? 'Yes' : 'Not established'}
              </Text>
              <Box className="backup-controls">
                <Button
                  disabled={busy}
                  aria-pressed={snapshotId === snapshot.id}
                  onClick={() => {
                    setSelectedSnapshot(snapshot);
                    setBrowse(null);
                    setBrowseError(null);
                    setBrowseOffset(0);
                    setBrowseLoading(true);
                    setReview(null);
                    setApproval(null);
                  }}
                >
                  Browse snapshot
                </Button>
                <Button
                  disabled={busy || Boolean(running)}
                  onClick={() =>
                    void action(async () => {
                      const value = await snapshotRequest(
                        { kind: 'verify', profile_id: profile.id, snapshot_id: snapshot.id },
                        'job',
                      );
                      if (alive.current) saveJob(value);
                    })
                  }
                >
                  Read back repository data
                </Button>
                <Button
                  disabled={busy || Boolean(running)}
                  onClick={() =>
                    void action(async () => {
                      const selection = await pickDirectory();
                      if (!selection || !alive.current) return;
                      const value = await snapshotRequest(
                        {
                          kind: 'restore_review',
                          profile_id: profile.id,
                          snapshot_id: snapshot.id,
                          selection_id: selection.id,
                        },
                        'review',
                      );
                      if (alive.current) showReview(value, 'restore', snapshot.rootIds);
                    })
                  }
                >
                  Review restore to new output…
                </Button>
              </Box>
            </List.Item>
          ))}
      </List.Root>
      {selectedSnapshot && (
        <chakra.section className="backup-entry-details" aria-label="Snapshot browser">
          <Heading as="h4" unstyled>
            Snapshot {selectedSnapshot.time}
          </Heading>
          <Text>
            The browser is read-only. Restore review currently covers the whole selected application
            snapshot, including its catalog; paging does not change that scope.
          </Text>
          {browseLoading && <Text role="status">Loading snapshot entries…</Text>}
          {browseError && (
            <Text className="error" role="alert">
              {browseError}
            </Text>
          )}
          {browse && (
            <>
              <Box className="backup-table">
                <Table.Root unstyled>
                  <Table.Caption unstyled captionSide="top">
                    Stored snapshot entries
                  </Table.Caption>
                  <Table.Header unstyled>
                    <Table.Row unstyled>
                      <Table.ColumnHeader unstyled>Relative path</Table.ColumnHeader>
                      <Table.ColumnHeader unstyled>Kind</Table.ColumnHeader>
                      <Table.ColumnHeader unstyled>Bytes</Table.ColumnHeader>
                      <Table.ColumnHeader unstyled>Stored content evidence</Table.ColumnHeader>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body unstyled>
                    {browse.entries.map((entry) => (
                      <Table.Row unstyled key={entry.id}>
                        <Table.Cell unstyled className="backup-path">
                          {entry.displayPath}
                        </Table.Cell>
                        <Table.Cell unstyled>{entry.kind}</Table.Cell>
                        <Table.Cell unstyled>{entry.bytes}</Table.Cell>
                        <Table.Cell unstyled className="backup-path">
                          {entry.digest ?? 'Not recorded'}
                        </Table.Cell>
                      </Table.Row>
                    ))}
                  </Table.Body>
                </Table.Root>
              </Box>
              <Box className="backup-controls">
                <Button
                  disabled={browseOffset === 0 || browseLoading}
                  onClick={() => {
                    setBrowse(null);
                    setBrowseLoading(true);
                    setBrowseOffset(Math.max(0, browseOffset - 200));
                  }}
                >
                  Previous snapshot entries
                </Button>
                <Text as="span">
                  Entries {browse.entries.length ? browseOffset + 1 : 0}–
                  {browseOffset + browse.entries.length} of {browse.total}
                </Text>
                <Button
                  disabled={browse.nextOffset === null || browseLoading}
                  onClick={() => {
                    setBrowse(null);
                    setBrowseLoading(true);
                    setBrowseOffset(browse.nextOffset ?? 0);
                  }}
                >
                  Next snapshot entries
                </Button>
              </Box>
            </>
          )}
          <Button
            onClick={() => {
              setSelectedSnapshot(null);
              setBrowse(null);
              setBrowseError(null);
            }}
          >
            Close snapshot browser
          </Button>
        </chakra.section>
      )}
      <details>
        <summary>Retention preview</summary>
        <Text>
          Keep all {snapshots.length} listed snapshots. No automatic pruning, forgetting or history
          expiry is enabled. Source removal does not authorize deleting repository history.
        </Text>
        <Text>
          This is the application's recorded snapshot history; an incomplete or unavailable
          repository is not treated as empty.
        </Text>
      </details>
      <Heading as="h4" unstyled>
        Snapshot jobs
      </Heading>
      <List.Root unstyled className="backup-job-list">
        {jobs.map((job) => (
          <List.Item unstyled key={job.id} className="backup-job">
            <Heading as="h5" unstyled>
              {job.action.replaceAll('_', ' ')} · {job.state.replaceAll('_', ' ')}
            </Heading>
            <Text role="status">{job.message}</Text>
            <Text>
              Phase: {job.phase.replaceAll('_', ' ')} · {job.files} files · {job.bytes} content
              bytes
            </Text>
            <Evidence value={job} />
            <CatalogRestore
              job={job}
              catalogId={profile.catalogId}
              disabled={busy || Boolean(running)}
              onActivityChange={onCatalogActivityChange}
            />
            {job.action === 'backup' && job.state === 'needs_recovery' && (
              <SnapshotRecoveryInspection
                key={job.id}
                job={job}
                disabled={busy || Boolean(running)}
                onInspect={(snapshotId) =>
                  void action(async () => {
                    setReview(null);
                    setApproval(null);
                    const value = await snapshotRequest(
                      {
                        kind: 'reconcile',
                        profile_id: profile.id,
                        job_id: job.id,
                        snapshot_id: snapshotId,
                      },
                      'job',
                    );
                    validJob(value, profile.id);
                    if (value.id === job.id || value.action !== 'reconcile')
                      throw new Error(
                        'Snapshot inspection returned an unexpected job. Inspect history.',
                      );
                    if (alive.current) saveJob(value);
                  })
                }
              />
            )}
            <Text>
              Source retained: {job.sourceRetained ? 'Yes' : 'Not established'} · Independent
              recovery:{' '}
              {job.independentRecoveryVerified
                ? 'Verified for the recorded scope'
                : 'Not established'}
            </Text>
            {job.snapshotId && <Text className="backup-path">Snapshot {job.snapshotId}</Text>}
            {active(job) && (
              <Button
                disabled={busy || job.state === 'cancel_requested'}
                onClick={() =>
                  void action(async () => {
                    const value = await snapshotRequest({ kind: 'cancel', job_id: job.id }, 'job');
                    validJob(value, profile.id, job);
                    if (alive.current) saveJob(value);
                  })
                }
              >
                Cancel snapshot job
              </Button>
            )}
          </List.Item>
        ))}
      </List.Root>
      <SnapshotSchedulePanel
        key={
          review?.action === 'backup'
            ? `schedule:${review.id}:${review.digest}`
            : 'no-backup-review'
        }
        review={review?.action === 'backup' ? review : null}
        capabilities={capabilities}
        busy={busy}
        running={Boolean(running)}
        schedule={schedule}
        onSave={(minutes, enabled, catchUp) =>
          void action(async () => {
            if (!review || review.action !== 'backup') return;
            const value = await snapshotRequest(
              {
                kind: 'schedule_save',
                profile_id: profile.id,
                review_id: review.id,
                digest: review.digest,
                interval_minutes: minutes,
                timezone: 'UTC',
                enabled,
                catch_up: catchUp,
              },
              'schedule',
            );
            if (!value || value.profileId !== profile.id || value.approvedDigest !== review.digest)
              throw new Error(
                'Schedule does not match the reviewed manifest. Refresh the profile.',
              );
            if (alive.current) setSchedule(value);
          })
        }
        onDisable={() =>
          void action(async () => {
            const value = await snapshotRequest(
              { kind: 'schedule_disable', profile_id: profile.id },
              'schedule',
            );
            if (value && value.profileId !== profile.id)
              throw new Error('Disabled schedule identity changed. Refresh the profile.');
            if (alive.current) setSchedule(value);
          })
        }
      />
    </chakra.section>
  );
}

export function SnapshotReviewPanel({
  review,
  roots,
  busy,
  approved,
  onApprove,
  onStart,
  onClose,
}: {
  review: SnapshotReview;
  roots: Pick<LibraryStatus['roots'][number], 'id' | 'label'>[];
  busy: boolean;
  approved: boolean;
  onApprove: () => void;
  onStart: () => void;
  onClose: () => void;
}) {
  const [acknowledged, setAcknowledged] = useState(false);
  const [offset, setOffset] = useState(0);
  const safeScope =
    review.sourceRetained &&
    review.items.length === review.files &&
    (review.action !== 'backup' || review.catalogIncluded);
  return (
    <chakra.section className="backup-review" aria-label="Snapshot action review">
      <Heading as="h4" unstyled>
        {review.action === 'backup'
          ? 'Review encrypted snapshot backup'
          : 'Review restore to a new protected output'}
      </Heading>
      <Text className="backup-path">Target: {review.targetDisplay}</Text>
      <List.Root unstyled aria-label="Reviewed snapshot source paths">
        {review.sourceDisplayPaths.map((path, index) => (
          <List.Item unstyled className="backup-path" key={index}>
            {path}
          </List.Item>
        ))}
      </List.Root>
      <Text>
        Approve the entire {review.items.length}-entry manifest, including entries on other pages.
        Manifest digest: <code>{review.digest}</code>
      </Text>
      <Box className="backup-table">
        <Table.Root unstyled>
          <Table.Caption unstyled captionSide="top">
            Exact reviewed snapshot files
          </Table.Caption>
          <Table.Header unstyled>
            <Table.Row unstyled>
              <Table.ColumnHeader unstyled>Native display path</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Bytes</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Full digest</Table.ColumnHeader>
            </Table.Row>
          </Table.Header>
          <Table.Body unstyled>
            {review.items.slice(offset, offset + 200).map((item, index) => (
              <Table.Row unstyled key={`${offset + index}:${item.displayPath}`}>
                <Table.Cell unstyled className="backup-path">
                  {item.displayPath}
                </Table.Cell>
                <Table.Cell unstyled>{item.bytes}</Table.Cell>
                <Table.Cell unstyled>
                  <code>{item.fullDigest}</code>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Root>
      </Box>
      <Box className="backup-controls">
        <Button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - 200))}>
          Previous snapshot plan entries
        </Button>
        <Text as="span">
          {Math.min(offset + 1, review.items.length)}–{Math.min(offset + 200, review.items.length)}{' '}
          of {review.items.length}
        </Text>
        <Button
          disabled={offset + 200 >= review.items.length}
          onClick={() => setOffset(offset + 200)}
        >
          Next snapshot plan entries
        </Button>
      </Box>
      <Text>
        {review.files} files · {review.bytes} content bytes · Catalog included:{' '}
        {review.catalogIncluded ? 'Yes' : 'No'} · Source retained:{' '}
        {review.sourceRetained ? 'Yes' : 'Not confirmed'}
      </Text>
      <Text>
        Roots:{' '}
        {review.rootIds
          .map((id) => roots.find((root) => root.id === id)?.label ?? `Unavailable root ${id}`)
          .join(' · ')}
      </Text>
      {review.issues.length > 0 && (
        <List.Root unstyled aria-label="Snapshot review issues">
          {review.issues.map((issue, index) => (
            <List.Item unstyled key={index}>
              {issue}
            </List.Item>
          ))}
        </List.Root>
      )}
      {!review.executionAvailable && (
        <Text role="status">
          {review.unavailableReason ?? 'This action has not passed its required native gates.'}
        </Text>
      )}
      {!safeScope && (
        <Text role="alert">
          This review does not confirm the required source-retaining catalog scope. Refresh and
          review again before any action.
        </Text>
      )}
      <Text>
        {review.action === 'backup'
          ? 'Encrypted repository data requires the recovery password. Inclusion, structural checks, data reads and restored-byte comparison remain separate evidence.'
          : 'Restore writes into a new output namespace. It does not overwrite an active library or automatically reopen a restored catalog. Root mappings and history must be reconciled before further actions.'}
      </Text>
      <label>
        <input
          type="checkbox"
          checked={acknowledged}
          disabled={busy || approved || !review.executionAvailable || !safeScope}
          onChange={(event) => setAcknowledged(event.target.checked)}
        />
        I reviewed this exact scope, target, listed issues and preservation policy
      </label>
      <Box className="backup-controls">
        {!approved ? (
          <Button
            disabled={busy || !review.executionAvailable || !safeScope || !acknowledged}
            onClick={onApprove}
          >
            Approve this snapshot action
          </Button>
        ) : (
          <Button
            className="primary"
            disabled={busy || !review.executionAvailable || !safeScope}
            onClick={onStart}
          >
            {review.action === 'backup' ? 'Start encrypted backup' : 'Start restore to new output'}
          </Button>
        )}
        <Button disabled={busy} onClick={onClose}>
          Close snapshot action review
        </Button>
      </Box>
    </chakra.section>
  );
}

function SnapshotSchedulePanel({
  review,
  capabilities,
  busy,
  running,
  schedule,
  onSave,
  onDisable,
}: {
  review: SnapshotReview | null;
  capabilities: SnapshotCapabilities | null;
  busy: boolean;
  running: boolean;
  schedule: SnapshotSchedule | null;
  onSave: (minutes: number, enabled: boolean, catchUp: boolean) => void;
  onDisable: () => void;
}) {
  const [interval, setInterval] = useState('1440');
  const [enabled, setEnabled] = useState(false);
  const [catchUp, setCatchUp] = useState(true);
  const [acknowledged, setAcknowledged] = useState(false);
  const validInterval =
    /^\d{1,5}$/.test(interval) && Number(interval) >= 15 && Number(interval) <= 10080;
  const available = Boolean(
    capabilities?.scheduleAvailable &&
    review?.executionAvailable &&
    review.catalogIncluded &&
    review.sourceRetained,
  );
  return (
    <chakra.section className="backup-review" aria-label="Snapshot schedule">
      <Heading as="h4" unstyled>
        Repeat an exact reviewed backup
      </Heading>
      <Text>{capabilities?.scheduleLimits ?? 'Checking scheduling capability…'}</Text>
      <Text>
        Schedules run only while the Stillharbour process is running. Fully-quit execution is
        unavailable. Only UTC elapsed intervals are offered.
      </Text>
      <Text>
        New or changed files, source roots, versions or endpoints require a new review. This consent
        never expands automatically to new content.
      </Text>
      {!review && (
        <Text>Review the backup scope above before configuring its repeating schedule.</Text>
      )}
      {review && (
        <Text>
          Current reviewed manifest: {review.files} files · {review.bytes} content bytes.
        </Text>
      )}
      <chakra.form
        onSubmit={(event) => {
          event.preventDefault();
          if (available && acknowledged && validInterval)
            onSave(Number(interval), enabled, catchUp);
        }}
      >
        <chakra.fieldset disabled={busy || running || !available}>
          <chakra.legend>Exact-scope recurring consent</chakra.legend>
          <label>
            Interval in minutes
            <Input
              inputMode="numeric"
              value={interval}
              maxLength={5}
              onChange={(event) => setInterval(event.target.value)}
            />
            <small>15–10080 minutes.</small>
          </label>
          <label>
            <input
              type="checkbox"
              checked={enabled}
              onChange={(event) => setEnabled(event.target.checked)}
            />
            Enable repeats of this exact reviewed scope while running
          </label>
          <label>
            <input
              type="checkbox"
              checked={catchUp}
              onChange={(event) => setCatchUp(event.target.checked)}
            />
            Allow one catch-up after sleep or restart when the reviewed manifest is unchanged
          </label>
          <Text>If unchecked, missed deadlines more than 30 seconds overdue are skipped.</Text>
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I approve repeating only this reviewed manifest; changes must return to review
          </label>
          <Button type="submit" disabled={!validInterval || !acknowledged}>
            Save snapshot schedule
          </Button>
        </chakra.fieldset>
      </chakra.form>
      {schedule && (
        <Text>
          {schedule.enabled ? 'Enabled' : 'Not enabled or authorization unavailable'} · Next due{' '}
          {schedule.nextDue || 'Not scheduled'} · {schedule.timezone} ·{' '}
          {schedule.dstPolicy.replaceAll('_', ' ')} · Last scheduled attempt{' '}
          {schedule.lastRun ?? 'Not recorded'}
          {!schedule.fullyQuitSupported && ' · Fully-quit execution unavailable'}
        </Text>
      )}
      {schedule && (
        <Button disabled={busy} onClick={onDisable}>
          Disable future snapshot repeats
        </Button>
      )}
    </chakra.section>
  );
}
