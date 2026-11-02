import { useEffect, useRef, useState, type ReactNode } from 'react';
import type {
  ComparisonEntry,
  ComparisonCategory,
  ComparisonPage,
  ComparisonRun,
  CompareMode,
  DirectorySelection,
  LibraryStatus,
  SyncProfile,
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';
import './backup.css';
import { ComparisonPreviews } from './ComparisonPreviews';
import { SyncWorkflow } from './SyncWorkflow';
import { SnapshotWorkflow } from './SnapshotWorkflow';

type ComparisonSource = Pick<
  LibraryStatus['roots'][number],
  'id' | 'label' | 'displayPath' | 'availability'
>;
const categories: ComparisonCategory[] = [
  'source_only',
  'destination_only',
  'verified_equal',
  'appears_unchanged_unverified',
  'different_content',
  'destination_edited_since_last_run',
  'uncertain',
  'blocked',
];
const summaryKeys = {
  source_only: 'sourceOnly',
  destination_only: 'destinationOnly',
  verified_equal: 'verifiedEqual',
  appears_unchanged_unverified: 'appearsUnchangedUnverified',
  different_content: 'differentContent',
  destination_edited_since_last_run: 'destinationEditedSinceLastRun',
  uncertain: 'uncertain',
  blocked: 'blocked',
} as const;

export function BackupScreen({
  status,
  navigationControl,
  jobsControl,
}: {
  status: LibraryStatus;
  navigationControl: ReactNode;
  jobsControl: ReactNode;
}) {
  const [profiles, setProfiles] = useState<SyncProfile[]>([]);
  const [selectedId, setSelectedId] = useState('');
  const [rebindFrom, setRebindFrom] = useState<SyncProfile | undefined>();
  const [profileError, setProfileError] = useState<string | null>(null);
  const [profileLoading, setProfileLoading] = useState(true);
  const [profileRefresh, setProfileRefresh] = useState(0);
  const [controlsBusy, setControlsBusy] = useState(false);
  const [workspace, setWorkspace] = useState<'safe_copy' | 'snapshots'>('safe_copy');
  useEffect(() => {
    let retired = false;
    void libraryData({ kind: 'backup_profiles' }, 'backup_profiles')
      .then((value) => {
        if (retired) return;
        if (!Array.isArray(value))
          throw new Error('Comparison profiles could not be read. Retry profiles.');
        setProfiles(value);
        setProfileError(null);
      })
      .catch((cause: unknown) => {
        if (!retired) setProfileError(errorMessage(cause));
      })
      .finally(() => {
        if (!retired) setProfileLoading(false);
      });
    return () => {
      retired = true;
    };
  }, [profileRefresh]);
  const profile = profiles.find((item) => item.id === selectedId);
  const registered = status.roots.find((root) => root.id === profile?.sourceRootId);
  const base: ComparisonSource | undefined = selectedId
    ? profile
      ? {
          id: profile.sourceRootId,
          label: profile.sourceLabel,
          displayPath: profile.sourceDisplayPath,
          availability: registered?.availability ?? 'unavailable',
        }
      : undefined
    : status.roots.find((root) => root.isBase);
  // Saved profiles retain their source even if the catalog's base changes.
  const workflowControl = (
    <div className="backup-mode-control" role="group" aria-label="Backup mode">
      <button
        aria-pressed={workspace === 'safe_copy'}
        disabled={controlsBusy}
        onClick={() => setWorkspace('safe_copy')}
      >
        Safe copy
      </button>
      <button
        aria-pressed={workspace === 'snapshots'}
        disabled={controlsBusy}
        onClick={() => setWorkspace('snapshots')}
      >
        Versioned backup
      </button>
    </div>
  );
  if (workspace === 'snapshots')
    return (
      <>
        <header className="toolbar">
          {navigationControl}
          <h1>Backup &amp; Sync</h1>
          <div className="header-actions">
            {workflowControl}
            {jobsControl}
          </div>
        </header>
        <SnapshotWorkflow status={status} onActivityChange={setControlsBusy} />
      </>
    );
  return (
    <BackupWorkspace
      key={JSON.stringify([
        selectedId,
        base?.id,
        base?.displayPath,
        base?.availability,
        registered?.displayPath,
        rebindFrom?.id,
      ])}
      rebindFrom={rebindFrom}
      base={base}
      currentSourcePath={registered?.displayPath ?? base?.displayPath}
      profile={profile}
      onActivityChange={setControlsBusy}
      onProfileSaved={(value) => {
        setProfiles((previous) => [...previous, value]);
        setSelectedId(value.id);
        setRebindFrom(undefined);
      }}
      profileControls={
        <section aria-label="Saved comparisons">
          <label>
            Saved comparison{' '}
            <select
              value={selectedId}
              disabled={controlsBusy || profileLoading}
              onChange={(event) => {
                setSelectedId(event.target.value);
                setRebindFrom(undefined);
              }}
            >
              <option value="">New comparison (current base library)</option>
              {profiles.map((item) => (
                <option value={item.id} key={item.id}>
                  {item.label} · {item.sourceLabel}
                </option>
              ))}
            </select>
          </label>
          {profileLoading && <p role="status">Loading saved comparisons…</p>}
          {profileError && (
            <p role="alert">
              {profileError}{' '}
              <button
                disabled={controlsBusy}
                onClick={() => {
                  setProfileLoading(true);
                  setProfileRefresh((value) => value + 1);
                }}
              >
                Retry profiles
              </button>
            </p>
          )}
          {selectedId && !profile && !profileLoading && (
            <p role="alert">
              The selected profile is unavailable. Choose another profile or explicitly start a new
              comparison.
            </p>
          )}
          {profile && (
            <button
              disabled={controlsBusy}
              onClick={() => {
                setRebindFrom(profile);
                setSelectedId('');
              }}
            >
              Review profile rebinding
            </button>
          )}
          {profile && (
            <p>
              Bound to root {profile.sourceRootId}. To change source or destination, choose New
              comparison and save a separate profile. Existing bindings are retained.
            </p>
          )}
        </section>
      }
      navigationControl={navigationControl}
      jobsControl={
        <>
          {workflowControl}
          {jobsControl}
        </>
      }
    />
  );
}

function BackupWorkspace({
  rebindFrom,
  base,
  currentSourcePath,
  profile,
  profileControls,
  onProfileSaved,
  onActivityChange,
  navigationControl,
  jobsControl,
}: {
  rebindFrom: SyncProfile | undefined;
  base: ComparisonSource | undefined;
  currentSourcePath: string | undefined;
  profile: SyncProfile | undefined;
  profileControls: ReactNode;
  onProfileSaved: (profile: SyncProfile) => void;
  onActivityChange: (busy: boolean) => void;
  navigationControl: ReactNode;
  jobsControl: ReactNode;
}) {
  const [destination, setDestination] = useState<DirectorySelection | null>(null);
  const [mode, setMode] = useState<CompareMode>('verified');
  const [run, setRun] = useState<ComparisonRun | null>(null);
  const [page, setPage] = useState<ComparisonPage | null>(null);
  const [offset, setOffset] = useState(0);
  const [category, setCategory] = useState<ComparisonCategory | ''>('');
  const [selected, setSelected] = useState<ComparisonEntry | null>(null);
  const [requestBusy, setBusy] = useState(false);
  const [transferBusy, setTransferBusy] = useState(false);
  const busy = requestBusy || transferBusy;
  const [profileLabel, setProfileLabel] = useState('');
  const [rebindReviewed, setRebindReviewed] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(true);
  const generation = useRef(0);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      generation.current += 1;
    };
  }, []);
  const running = run?.state === 'comparing';
  const runId = run?.id;
  const profileId = profile?.id;
  useEffect(() => {
    onActivityChange(busy || running);
  }, [busy, running, onActivityChange]);
  useEffect(() => {
    let retired = false;
    const token = generation.current;
    void libraryData({ kind: 'backup_comparison_latest' }, 'backup_comparison_latest')
      .then((latest) => {
        if (
          !retired &&
          token === generation.current &&
          latest !== null &&
          latest?.sourceRootId === base?.id &&
          latest.sourceDisplayPath === currentSourcePath &&
          latest?.profileId === profileId
        )
          setRun(latest);
      })
      .catch((cause: unknown) => {
        if (!retired && token === generation.current) setError(errorMessage(cause));
      });
    return () => {
      retired = true;
    };
  }, [base?.id, currentSourcePath, profileId]);
  useEffect(() => {
    if (!runId || !running) return;
    let retired = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const next = await libraryData(
          { kind: 'backup_comparison', run_id: runId },
          'backup_comparison',
        );
        if (retired) return;
        if (next.id !== runId || next.sourceRootId !== base?.id || next.profileId !== profileId)
          throw new Error('Comparison identity changed. Start a fresh comparison.');
        setRun(next);
        if (next.state === 'comparing') timer = setTimeout(() => void poll(), 500);
      } catch (cause) {
        if (!retired) {
          setError(errorMessage(cause));
          timer = setTimeout(() => void poll(), 2000);
        }
      }
    };
    timer = setTimeout(() => void poll(), 250);
    return () => {
      retired = true;
      clearTimeout(timer);
    };
  }, [runId, running, base?.id, profileId]);

  useEffect(() => {
    if (!run || running || run.state === 'failed' || run.state === 'canceled') return;
    let retired = false;
    void libraryData(
      { kind: 'backup_comparison_page', run_id: run.id, offset, ...(category ? { category } : {}) },
      'backup_comparison_page',
    )
      .then((next) => {
        if (!retired) {
          if (next.runId !== run.id) {
            setError('Comparison page identity changed. Recompare.');
            return;
          }
          setPage(next);
        }
      })
      .catch((cause: unknown) => {
        if (!retired) setError(errorMessage(cause));
      });
    return () => {
      retired = true;
    };
  }, [run, running, offset, category]);

  async function action(work: () => Promise<void>) {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      if (alive.current) setError(errorMessage(cause));
    } finally {
      if (alive.current) setBusy(false);
    }
  }
  function retire() {
    generation.current += 1;
    setRun(null);
    setPage(null);
    setOffset(0);
    setCategory('');
    setSelected(null);
  }

  return (
    <>
      <header className="toolbar">
        {navigationControl}
        <h1>Backup &amp; Sync</h1>
        <div className="header-actions">{jobsControl}</div>
      </header>
      <div className="backup-workspace">
        {profileControls}
        {rebindFrom && (
          <section aria-label="Profile rebinding review">
            <h2>Review a new immutable binding</h2>
            <p className="backup-path">
              Previous source: {rebindFrom.sourceLabel} · {rebindFrom.sourceDisplayPath} ·{' '}
              {rebindFrom.sourceRootId}
            </p>
            <p className="backup-path">Previous destination: {rebindFrom.destinationDisplayPath}</p>
            <p>
              The cards below show the current base and newly selected destination. The previous
              profile and historical observations are retained. A fresh comparison is required; no
              destination content is adopted or removed.
            </p>
            <label>
              <input
                type="checkbox"
                checked={rebindReviewed}
                onChange={(event) => setRebindReviewed(event.target.checked)}
              />
              I reviewed the previous and new source/destination bindings
            </label>
          </section>
        )}
        <h2>Compare before copying</h2>
        <p>
          Library → Destination. Comparison leaves both folders unchanged. Review transfer
          capabilities, proposed actions and any limits below before approving a copy.
        </p>
        <div className="backup-scope">
          <section aria-label="Fixed source library">
            <h3>Library (source)</h3>
            <strong>{base?.label ?? 'No base library configured'}</strong>
            <p className="backup-path">{base?.displayPath}</p>
            <p>
              {base?.availability ?? 'unavailable'} · All regular files, including hidden files,
              sidecars and unknown formats.
            </p>
            <small>Root: {base?.id ?? 'none'}</small>
          </section>
          <section aria-label="Comparison destination">
            <h3>Destination</h3>
            <p className="backup-path">
              {profile?.destinationDisplayPath ??
                destination?.displayPath ??
                'Choose a separate folder.'}
            </p>
            <button
              disabled={busy || running || !base || Boolean(profile)}
              onClick={() =>
                void action(async () => {
                  const token = ++generation.current;
                  const selected = await pickDirectory();
                  if (alive.current && token === generation.current && selected) {
                    retire();
                    setDestination(selected);
                    setRebindReviewed(false);
                  }
                })
              }
            >
              Choose destination
            </button>
            <p>
              Only the proposed Stillharbour dataset is compared. Other files in this folder remain
              outside the scope. Existing data is not adopted by selecting it.
            </p>
          </section>
        </div>
        {!profile && (
          <div className="backup-controls">
            <label>
              Comparison profile name{' '}
              <input
                value={profileLabel}
                maxLength={80}
                disabled={busy || running}
                onChange={(event) => setProfileLabel(event.target.value)}
              />
            </label>
            <button
              disabled={
                busy ||
                running ||
                !destination?.identityAvailable ||
                !profileLabel.trim() ||
                Boolean(rebindFrom && !rebindReviewed) ||
                base?.availability !== 'online'
              }
              onClick={() =>
                void action(async () => {
                  if (!destination || !base) return;
                  const token = generation.current;
                  const value = await libraryData(
                    rebindFrom
                      ? {
                          kind: 'backup_profile_rebind',
                          prior_id: rebindFrom.id,
                          selection_id: destination.id,
                          label: profileLabel,
                          reviewed: rebindReviewed,
                        }
                      : {
                          kind: 'backup_profile_save',
                          selection_id: destination.id,
                          label: profileLabel,
                        },
                    'backup_profile',
                  );
                  if (!alive.current || token !== generation.current) return;
                  if (value.sourceRootId !== base.id)
                    throw new Error(
                      'The base library changed. Choose the destination and review the new source again.',
                    );
                  onProfileSaved(value);
                })
              }
            >
              Save comparison profile
            </button>
            <p>
              Stores this read-only source and destination binding in the catalog. It saves no
              approval and creates no destination files.
            </p>
          </div>
        )}
        <div className="backup-controls">
          <label>
            Comparison{' '}
            <select
              value={mode}
              disabled={busy || running}
              onChange={(event) => {
                retire();
                setMode(event.target.value as CompareMode);
              }}
            >
              <option value="verified">Verified (complete bytes)</option>
              <option value="quick">Quick (content not checked)</option>
            </select>
          </label>
          <button
            className="primary"
            disabled={
              busy ||
              running ||
              !(profile || destination?.identityAvailable) ||
              base?.availability !== 'online'
            }
            onClick={() =>
              void action(async () => {
                if ((!destination && !profile) || !base) return;
                retire();
                const token = generation.current;
                const result = await libraryData(
                  profile
                    ? { kind: 'backup_profile_compare', profile_id: profile.id, mode }
                    : { kind: 'backup_compare_start', selection_id: destination!.id, mode },
                  'backup_comparison',
                );
                if (!alive.current || token !== generation.current) return;
                if (result.sourceRootId !== base.id || result.profileId !== profileId)
                  throw new Error(
                    'The base library changed. Reopen Backup & Sync and compare again.',
                  );
                setRun(result);
              })
            }
          >
            Compare
          </button>
          {running && (
            <button
              disabled={busy}
              onClick={() =>
                void action(async () => {
                  if (!run) return;
                  await libraryData(
                    { kind: 'backup_compare_cancel', run_id: run.id },
                    'backup_comparison',
                  );
                })
              }
            >
              Cancel comparison
            </button>
          )}
        </div>
        <p>
          Comparison is not a backup or an approval. Preparation, copying and recovery each require
          a separate action review.
        </p>
        {!base && <p role="status">Configure a base library before comparing.</p>}
        {base && base.availability !== 'online' && (
          <p role="status">
            The source is unavailable. It is not an empty library; reconnect it before comparing.
          </p>
        )}
        {destination && !destination.identityAvailable && (
          <p role="alert">
            This destination cannot supply the required native identity. Choose a different folder.
          </p>
        )}
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {run && (
          <section aria-label="Comparison review">
            <h3>{run.state.replaceAll('_', ' ')}</h3>
            <p role="status">{run.message}</p>
            <p className="backup-path">Dataset: {run.datasetDisplayPath}</p>
            <p>
              {run.historyMessage} Prior observation:{' '}
              {run.baselineId ?? 'none (initial comparison)'}
            </p>
            <p>
              {run.entries} entries observed · {run.bytesRead} content bytes read
            </p>
            <p>
              Modified timestamps are filesystem observations in UTC, with filesystem-dependent
              precision; they are separate from capture and birth dates.
            </p>
            {run.summary && (
              <ul aria-label="Comparison totals" className="backup-totals">
                {categories.map((finding) => (
                  <li key={finding}>
                    {finding.replaceAll('_', ' ')}: {run.summary![summaryKeys[finding]]}
                  </li>
                ))}
              </ul>
            )}
            <p>
              Reads use the shared application I/O budget. Filesystem metadata, other applications
              and physical disk traffic are outside that byte cap. Reviews are session-only and
              expire on restart or a new comparison.
            </p>
            {run.issues.length > 0 && (
              <details open>
                <summary>Issues ({run.issues.length}; first 200 shown)</summary>
                <ul>
                  {run.issues.map((issue, index) => (
                    <li key={index}>
                      {issue.side && `${issue.side}: `}
                      <span className="backup-path">{issue.displayPath}</span>: {issue.message}
                    </li>
                  ))}
                </ul>
              </details>
            )}
            {!running && run.state !== 'failed' && run.state !== 'canceled' && (
              <label>
                Finding filter{' '}
                <select
                  value={category}
                  onChange={(event) => {
                    setPage(null);
                    setSelected(null);
                    setOffset(0);
                    setCategory(event.target.value as ComparisonCategory | '');
                  }}
                >
                  <option value="">All findings</option>
                  {categories.map((finding) => (
                    <option key={finding} value={finding}>
                      Show {finding.replaceAll('_', ' ')}
                    </option>
                  ))}
                </select>
              </label>
            )}
            {page && (
              <>
                <ComparisonTable
                  entries={page.entries}
                  selectedId={selected?.id}
                  disabled={busy}
                  onSelect={setSelected}
                />
                {selected && (
                  <section aria-label="Entry details" className="backup-entry-details">
                    <h4 className="backup-path">{selected.displayPath}</h4>
                    <p>
                      {selected.category.replaceAll('_', ' ')} ·{' '}
                      {selected.proposal.replaceAll('_', ' ')}
                    </p>
                    <p>{selected.evidence}</p>
                    <p>
                      Potential same-stem companions (up to 32; each retains its own evidence):{' '}
                      {selected.companions?.join(' · ') || 'none displayed'}. Name association does
                      not prove bundle completeness or redundancy.
                    </p>
                    <dl>
                      {(['source', 'destination'] as const).map((side) => (
                        <div key={side}>
                          <dt>{side === 'source' ? 'Source' : 'Destination'}</dt>
                          <dd>
                            {selected[`${side}Kind`] ??
                              'Not observed; absence is established only by a complete inventory'}{' '}
                            · {selected[`${side}Bytes`] ?? 'unknown'} bytes ·{' '}
                            {selected[`${side}Modified`] ?? 'unknown modified time'}
                          </dd>
                        </div>
                      ))}
                    </dl>
                    <p>
                      Observed metadata and comparison evidence only. This review does not establish
                      companion completeness, metadata fidelity or recovery.
                    </p>
                    <ComparisonPreviews
                      key={`${run.id}:${selected.id}`}
                      runId={run.id}
                      entryId={selected.id}
                    />
                    <button onClick={() => setSelected(null)}>Close entry details</button>
                  </section>
                )}
                <div className="backup-controls">
                  <button
                    disabled={offset === 0}
                    onClick={() => {
                      setPage(null);
                      setSelected(null);
                      setOffset(Math.max(0, offset - 200));
                    }}
                  >
                    Previous
                  </button>
                  <span>
                    Entries {page.entries.length ? offset + 1 : 0}–{offset + page.entries.length}
                    {page.total !== undefined && ` of ${page.total}`}
                  </span>
                  <button
                    disabled={page.nextOffset === null}
                    onClick={() => {
                      setPage(null);
                      setSelected(null);
                      setOffset(page.nextOffset ?? 0);
                    }}
                  >
                    Next
                  </button>
                </div>
              </>
            )}
          </section>
        )}
        <SyncWorkflow
          key={run?.id ?? 'no-comparison'}
          run={run}
          selected={selected}
          onActivityChange={setTransferBusy}
        />
      </div>
    </>
  );
}

function ComparisonTable({
  entries,
  selectedId,
  disabled,
  onSelect,
}: {
  entries: ComparisonEntry[];
  selectedId: string | undefined;
  disabled: boolean;
  onSelect: (entry: ComparisonEntry) => void;
}) {
  return (
    <div className="backup-table">
      <table>
        <caption>Comparison findings and proposed actions</caption>
        <thead>
          <tr>
            <th>Relative path</th>
            <th>Source bytes / modified (UTC)</th>
            <th>Destination bytes / modified (UTC)</th>
            <th>Finding</th>
            <th>Proposed action / evidence</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.id}>
              <td className="backup-path">
                <button
                  disabled={disabled}
                  aria-pressed={selectedId === entry.id}
                  onClick={() => onSelect(entry)}
                >
                  {entry.displayPath}
                </button>
              </td>
              <td>
                {entry.sourceBytes ?? '—'}
                <br />
                {entry.sourceModified ?? 'unknown'}
              </td>
              <td>
                {entry.destinationBytes ?? '—'}
                <br />
                {entry.destinationModified ?? 'unknown'}
              </td>
              <td>{entry.category.replaceAll('_', ' ')}</td>
              <td>
                {entry.proposal.replaceAll('_', ' ')}
                <p>{entry.evidence}</p>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
