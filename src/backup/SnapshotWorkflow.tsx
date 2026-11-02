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
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';
import { SnapshotProfileForm } from './SnapshotProfileForm';
import { snapshotRequest } from './snapshotIpc';
import { SnapshotRecoveryInspection } from './SnapshotRecoveryInspection';

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
      <div>
        <dt>Included in snapshot</dt>
        <dd>{value.included ? 'Recorded' : 'Not established'}</dd>
      </div>
      <div>
        <dt>Repository structure checked</dt>
        <dd>{value.structuralChecked ? 'Recorded' : 'Not established'}</dd>
      </div>
      <div>
        <dt>Stored data read and verified</dt>
        <dd>{value.dataReadVerified ? 'Recorded' : 'Not established'}</dd>
      </div>
      <div>
        <dt>Restored and byte-compared</dt>
        <dd>{value.restoredCompared ? 'Recorded' : 'Not established'}</dd>
      </div>
    </dl>
  );
}

export function SnapshotWorkflow({
  status,
  onActivityChange,
}: {
  status: LibraryStatus;
  onActivityChange: (active: boolean) => void;
}) {
  const [capabilities, setCapabilities] = useState<SnapshotCapabilities | null>(null);
  const [profiles, setProfiles] = useState<SnapshotProfile[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [showSetup, setShowSetup] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  const refreshProfileViews = useCallback(() => setRefresh((value) => value + 1), []);
  const [creationBusy, setCreationBusy] = useState(false);
  const [profileBusy, setProfileBusy] = useState(false);
  const locked = creationBusy || profileBusy;
  useEffect(() => {
    onActivityChange(locked);
  }, [locked, onActivityChange]);
  useEffect(() => {
    let retired = false;
    void Promise.all([
      snapshotRequest({ kind: 'capabilities' }, 'capabilities'),
      snapshotRequest({ kind: 'profiles' }, 'profiles'),
    ])
      .then(([caps, saved]) => {
        if (!Array.isArray(saved)) throw new Error('Snapshot profiles could not be read.');
        if (!retired) {
          setCapabilities(caps);
          setProfiles(saved);
          setError(null);
        }
      })
      .catch((cause: unknown) => {
        if (!retired) setError(errorMessage(cause));
      })
      .finally(() => {
        if (!retired) setLoading(false);
      });
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
    <div className="backup-workspace">
      <h2>Encrypted versioned backup</h2>
      <p>
        Snapshots retain earlier versions in a restic repository. They need the repository recovery
        password and a verified restore to establish recoverability.
      </p>
      <p>
        This is separate from a readable safe copy. Removing a source does not automatically delete
        earlier snapshots.
      </p>
      {loading && <p role="status">Loading snapshot profiles…</p>}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {capabilities?.unavailableReason && <p role="status">{capabilities.unavailableReason}</p>}
      <div className="backup-controls">
        <label>
          Snapshot profile
          <select
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
          </select>
        </label>
        <button
          disabled={locked || loading}
          onClick={() => {
            setSelectedId('');
            setShowSetup(true);
          }}
        >
          New snapshot profile
        </button>
        <button
          disabled={locked}
          onClick={() => {
            setLoading(true);
            setRefresh((current) => current + 1);
          }}
        >
          Refresh snapshot profiles
        </button>
      </div>
      {!loading && !profiles.length && !error && (
        <p>
          No encrypted snapshot profiles are recorded. Create one using the current base library.
        </p>
      )}
      {selectedId && !selected && !loading && (
        <p role="alert">
          This profile is unavailable. Refresh profiles; a missing profile is not an empty
          repository.
        </p>
      )}
      {showSetup && (
        <SnapshotProfileForm
          key={JSON.stringify(
            status.roots.map((root) => [root.id, root.isBase, root.displayPath, root.availability]),
          )}
          status={status}
          capabilities={capabilities}
          disabled={profileBusy}
          onActivityChange={setCreationBusy}
          onClose={() => setShowSetup(false)}
          onSaved={(value) => {
            setProfiles((current) => [
              value,
              ...current.filter((profile) => profile.id !== value.id),
            ]);
            setSelectedId(value.id);
            setShowSetup(false);
          }}
        />
      )}
      {selected && (
        <SnapshotProfileWorkspace
          key={JSON.stringify([selected.id, sourceBinding])}
          profile={selected}
          status={status}
          capabilities={capabilities}
          onActivityChange={setProfileBusy}
          onRefreshProfiles={refreshProfileViews}
          onProfileUpdated={(value) => {
            setProfiles((current) =>
              current.some((profile) => profile.id === value.id)
                ? current.map((profile) => (profile.id === value.id ? value : profile))
                : [value, ...current],
            );
            setSelectedId(value.id);
          }}
        />
      )}
      {capabilities && (
        <aside className="backup-helper-limits">
          <h3>Transfer and schedule limits</h3>
          <p>{capabilities.helperLimits}</p>
          <p>{capabilities.scheduleLimits}</p>
          <p>
            Upload progress, a structural check and a restored-byte comparison are separate evidence
            levels. None alone proves an independent failure domain.
          </p>
        </aside>
      )}
    </div>
  );
}

function SnapshotProfileWorkspace({
  profile,
  status,
  capabilities,
  onActivityChange,
  onProfileUpdated,
  onRefreshProfiles,
}: {
  profile: SnapshotProfile;
  status: LibraryStatus;
  capabilities: SnapshotCapabilities | null;
  onActivityChange: (active: boolean) => void;
  onProfileUpdated: (value: SnapshotProfile) => void;
  onRefreshProfiles: () => void;
}) {
  const [snapshots, setSnapshots] = useState<SnapshotRecord[]>([]);
  const [jobs, setJobs] = useState<SnapshotJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [checkingHistory, setCheckingHistory] = useState(true);
  const [refresh, setRefresh] = useState(0);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
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
    onActivityChange(busy || Boolean(running));
  }, [busy, running, onActivityChange]);
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
    <section aria-label="Snapshot profile workspace" aria-busy={busy} className="backup-operation">
      <h3>{profile.label}</h3>
      <div className="backup-scope">
        <section aria-label="Snapshot source roots">
          <h4>Bound source roots</h4>
          <ul>
            {profile.rootIds.map((id) => {
              const root = status.roots.find((root) => root.id === id);
              return (
                <li key={id}>
                  <strong>{root?.label ?? 'Unavailable root'}</strong> · Last observed access:{' '}
                  {root?.availability ?? 'not registered'}
                  <p className="backup-path">{root?.displayPath ?? id}</p>
                </li>
              );
            })}
          </ul>
          <p>
            This profile keeps these root identities when the base library changes. Offline roots
            remain part of the expected scope.
          </p>
        </section>
        <section aria-label="Snapshot repository">
          <h4>Encrypted repository</h4>
          <p className="backup-path">{profile.destinationDisplay}</p>
          <p>
            {profile.protocol} · {profile.status.replaceAll('_', ' ')}
          </p>
          <p>Repository identity: {profile.repositoryId ?? 'Not established yet'}</p>
          {profile.hostFingerprint && (
            <p className="backup-path">Configured host fingerprint: {profile.hostFingerprint}</p>
          )}
          <p>
            Credentials:{' '}
            {profile.credentialsPresent
              ? 'Present in this device’s vault'
              : 'Unavailable or locked — unlock before starting'}
          </p>
          {profile.insecureTransport && (
            <p>Plain FTP credentials and connection metadata are unencrypted.</p>
          )}
        </section>
      </div>
      <div className="backup-controls">
        <button
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
        </button>
        <button
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
        </button>
        <button
          disabled={busy || checkingHistory}
          onClick={() => {
            setLoading(true);
            setCheckingHistory(true);
            setRefresh((current) => current + 1);
            onRefreshProfiles();
          }}
        >
          Refresh snapshot history
        </button>
      </div>
      {profile.protocol === 'local' && (
        <p>
          Reauthorization requires a fresh native selection of this profile's repository parent. A
          different storage identity is not silently adopted.
        </p>
      )}
      <details className="backup-key-recovery">
        <summary>Repository password recovery</summary>
        <p>
          Encrypted snapshots require the repository password. Keep an independent recovery copy;
          losing the secret can make the backup unreadable.
        </p>
        <p>
          Export writes an unencrypted recovery key into a new file selected through the native
          dialog. Store that file separately and protect it from disclosure.
        </p>
        <p>
          Unlock uses a native secret dialog and creates a new profile credential binding. Previous
          credentials and repository history remain retained. Enter the existing repository
          password; choosing another password does not recover encrypted data.
        </p>
        <div className="backup-controls">
          <button
            disabled={busy || Boolean(running) || !capabilities?.credentialsAvailable}
            onClick={() => void action(() => unlock('repository'))}
          >
            Unlock repository password…
          </button>
          {profile.protocol !== 'local' && (
            <button
              disabled={busy || Boolean(running) || !capabilities?.credentialsAvailable}
              onClick={() => void action(() => unlock('transport'))}
            >
              Unlock remote credentials…
            </button>
          )}
        </div>
        <button
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
        </button>
        {exportMessage && <p role="status">{exportMessage}</p>}
      </details>
      {!profile.credentialsPresent && (
        <p role="status">
          One or more required credentials are unavailable or locked on this device. A successful
          old upload cannot substitute for the repository recovery password.
        </p>
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
        <p className="error" role="alert">
          {error}
        </p>
      )}
      {historyError && (
        <p className="error" role="alert">
          {historyError}
        </p>
      )}
      {loading && <p role="status">Loading snapshots and job history…</p>}
      {!loading && !snapshots.length && !historyError && (
        <p>
          No application snapshots are recorded for this profile. This does not prove that an
          unavailable repository is empty.
        </p>
      )}
      <h4>Snapshot history</h4>
      <ul className="backup-job-list">
        {snapshots.map((snapshot) => (
          <li className="backup-job" key={snapshot.id}>
            <h5>
              {snapshot.time} · {snapshot.files} files · {snapshot.bytes} content bytes
            </h5>
            <p className="backup-path">Snapshot {snapshot.id}</p>
            <Evidence value={snapshot} />
            <p>
              Catalog included: {snapshot.catalogIncluded ? 'Yes' : 'Not established'} · Source
              retained: {snapshot.sourceRetained ? 'Yes' : 'Not established'}
            </p>
            <div className="backup-controls">
              <button
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
              </button>
              <button
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
              </button>
              <button
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
              </button>
            </div>
          </li>
        ))}
      </ul>
      {selectedSnapshot && (
        <section className="backup-entry-details" aria-label="Snapshot browser">
          <h4>Snapshot {selectedSnapshot.time}</h4>
          <p>
            The browser is read-only. Restore review currently covers the whole selected application
            snapshot, including its catalog; paging does not change that scope.
          </p>
          {browseLoading && <p role="status">Loading snapshot entries…</p>}
          {browseError && (
            <p className="error" role="alert">
              {browseError}
            </p>
          )}
          {browse && (
            <>
              <div className="backup-table">
                <table>
                  <caption>Stored snapshot entries</caption>
                  <thead>
                    <tr>
                      <th>Relative path</th>
                      <th>Kind</th>
                      <th>Bytes</th>
                      <th>Stored content evidence</th>
                    </tr>
                  </thead>
                  <tbody>
                    {browse.entries.map((entry) => (
                      <tr key={entry.id}>
                        <td className="backup-path">{entry.displayPath}</td>
                        <td>{entry.kind}</td>
                        <td>{entry.bytes}</td>
                        <td className="backup-path">{entry.digest ?? 'Not recorded'}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="backup-controls">
                <button
                  disabled={browseOffset === 0 || browseLoading}
                  onClick={() => {
                    setBrowse(null);
                    setBrowseLoading(true);
                    setBrowseOffset(Math.max(0, browseOffset - 200));
                  }}
                >
                  Previous snapshot entries
                </button>
                <span>
                  Entries {browse.entries.length ? browseOffset + 1 : 0}–
                  {browseOffset + browse.entries.length} of {browse.total}
                </span>
                <button
                  disabled={browse.nextOffset === null || browseLoading}
                  onClick={() => {
                    setBrowse(null);
                    setBrowseLoading(true);
                    setBrowseOffset(browse.nextOffset ?? 0);
                  }}
                >
                  Next snapshot entries
                </button>
              </div>
            </>
          )}
          <button
            onClick={() => {
              setSelectedSnapshot(null);
              setBrowse(null);
              setBrowseError(null);
            }}
          >
            Close snapshot browser
          </button>
        </section>
      )}
      <details>
        <summary>Retention preview</summary>
        <p>
          Keep all {snapshots.length} listed snapshots. No automatic pruning, forgetting or history
          expiry is enabled. Source removal does not authorize deleting repository history.
        </p>
        <p>
          This is the application's recorded snapshot history; an incomplete or unavailable
          repository is not treated as empty.
        </p>
      </details>
      <h4>Snapshot jobs</h4>
      <ul className="backup-job-list">
        {jobs.map((job) => (
          <li key={job.id} className="backup-job">
            <h5>
              {job.action.replaceAll('_', ' ')} · {job.state.replaceAll('_', ' ')}
            </h5>
            <p role="status">{job.message}</p>
            <p>
              Phase: {job.phase.replaceAll('_', ' ')} · {job.files} files · {job.bytes} content
              bytes
            </p>
            <Evidence value={job} />
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
            <p>
              Source retained: {job.sourceRetained ? 'Yes' : 'Not established'} · Independent
              recovery:{' '}
              {job.independentRecoveryVerified
                ? 'Verified for the recorded scope'
                : 'Not established'}
            </p>
            {job.snapshotId && <p className="backup-path">Snapshot {job.snapshotId}</p>}
            {active(job) && (
              <button
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
              </button>
            )}
          </li>
        ))}
      </ul>
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
    </section>
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
    <section className="backup-review" aria-label="Snapshot action review">
      <h4>
        {review.action === 'backup'
          ? 'Review encrypted snapshot backup'
          : 'Review restore to a new protected output'}
      </h4>
      <p className="backup-path">Target: {review.targetDisplay}</p>
      <ul aria-label="Reviewed snapshot source paths">
        {review.sourceDisplayPaths.map((path, index) => (
          <li className="backup-path" key={index}>
            {path}
          </li>
        ))}
      </ul>
      <p>
        Approve the entire {review.items.length}-entry manifest, including entries on other pages.
        Manifest digest: <code>{review.digest}</code>
      </p>
      <div className="backup-table">
        <table>
          <caption>Exact reviewed snapshot files</caption>
          <thead>
            <tr>
              <th>Native display path</th>
              <th>Bytes</th>
              <th>Full digest</th>
            </tr>
          </thead>
          <tbody>
            {review.items.slice(offset, offset + 200).map((item, index) => (
              <tr key={`${offset + index}:${item.displayPath}`}>
                <td className="backup-path">{item.displayPath}</td>
                <td>{item.bytes}</td>
                <td>
                  <code>{item.fullDigest}</code>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="backup-controls">
        <button disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - 200))}>
          Previous snapshot plan entries
        </button>
        <span>
          {Math.min(offset + 1, review.items.length)}–{Math.min(offset + 200, review.items.length)}{' '}
          of {review.items.length}
        </span>
        <button
          disabled={offset + 200 >= review.items.length}
          onClick={() => setOffset(offset + 200)}
        >
          Next snapshot plan entries
        </button>
      </div>
      <p>
        {review.files} files · {review.bytes} content bytes · Catalog included:{' '}
        {review.catalogIncluded ? 'Yes' : 'No'} · Source retained:{' '}
        {review.sourceRetained ? 'Yes' : 'Not confirmed'}
      </p>
      <p>
        Roots:{' '}
        {review.rootIds
          .map((id) => roots.find((root) => root.id === id)?.label ?? `Unavailable root ${id}`)
          .join(' · ')}
      </p>
      {review.issues.length > 0 && (
        <ul aria-label="Snapshot review issues">
          {review.issues.map((issue, index) => (
            <li key={index}>{issue}</li>
          ))}
        </ul>
      )}
      {!review.executionAvailable && (
        <p role="status">
          {review.unavailableReason ?? 'This action has not passed its required native gates.'}
        </p>
      )}
      {!safeScope && (
        <p role="alert">
          This review does not confirm the required source-retaining catalog scope. Refresh and
          review again before any action.
        </p>
      )}
      <p>
        {review.action === 'backup'
          ? 'Encrypted repository data requires the recovery password. Inclusion, structural checks, data reads and restored-byte comparison remain separate evidence.'
          : 'Restore writes into a new output namespace. It does not overwrite an active library or automatically reopen a restored catalog. Root mappings and history must be reconciled before further actions.'}
      </p>
      <label>
        <input
          type="checkbox"
          checked={acknowledged}
          disabled={busy || approved || !review.executionAvailable || !safeScope}
          onChange={(event) => setAcknowledged(event.target.checked)}
        />
        I reviewed this exact scope, target, listed issues and preservation policy
      </label>
      <div className="backup-controls">
        {!approved ? (
          <button
            disabled={busy || !review.executionAvailable || !safeScope || !acknowledged}
            onClick={onApprove}
          >
            Approve this snapshot action
          </button>
        ) : (
          <button
            className="primary"
            disabled={busy || !review.executionAvailable || !safeScope}
            onClick={onStart}
          >
            {review.action === 'backup' ? 'Start encrypted backup' : 'Start restore to new output'}
          </button>
        )}
        <button disabled={busy} onClick={onClose}>
          Close snapshot action review
        </button>
      </div>
    </section>
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
    <section className="backup-review" aria-label="Snapshot schedule">
      <h4>Repeat an exact reviewed backup</h4>
      <p>{capabilities?.scheduleLimits ?? 'Checking scheduling capability…'}</p>
      <p>
        Schedules run only while the Stillharbour process is running. Fully-quit execution is
        unavailable. Only UTC elapsed intervals are offered.
      </p>
      <p>
        New or changed files, source roots, versions or endpoints require a new review. This consent
        never expands automatically to new content.
      </p>
      {!review && <p>Review the backup scope above before configuring its repeating schedule.</p>}
      {review && (
        <p>
          Current reviewed manifest: {review.files} files · {review.bytes} content bytes.
        </p>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (available && acknowledged && validInterval)
            onSave(Number(interval), enabled, catchUp);
        }}
      >
        <fieldset disabled={busy || running || !available}>
          <legend>Exact-scope recurring consent</legend>
          <label>
            Interval in minutes
            <input
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
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I approve repeating only this reviewed manifest; changes must return to review
          </label>
          <button type="submit" disabled={!validInterval || !acknowledged}>
            Save snapshot schedule
          </button>
        </fieldset>
      </form>
      {schedule && (
        <p>
          {schedule.enabled ? 'Enabled' : 'Not enabled or authorization unavailable'} · Next due{' '}
          {schedule.nextDue || 'Not scheduled'} · {schedule.timezone} ·{' '}
          {schedule.dstPolicy.replaceAll('_', ' ')} · Last run {schedule.lastRun ?? 'Not recorded'}
          {!schedule.fullyQuitSupported && ' · Fully-quit execution unavailable'}
        </p>
      )}
      {schedule && (
        <button disabled={busy} onClick={onDisable}>
          Disable future snapshot repeats
        </button>
      )}
    </section>
  );
}
