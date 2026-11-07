import { PageHeader } from '@/components/PageHeader';
import { Field, Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
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
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';
import '../../backup/backup.css';
import { ComparisonPreviews } from '@/components/ComparisonPreviews';
import { SyncWorkflow } from '@/components/SyncWorkflow';
import { useCatalogSwitchState } from '../../backup/catalogSwitchState';
import { CatalogSwitchStopped } from '@/components/CatalogSwitchStopped';
import { UtilityWelcome } from '@/components/UtilityWelcome';
import { UtilityHistory } from '@/components/UtilityHistory';
import { formatScanDate } from '../../utils/format-scan-date';

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

export function SyncScreen(props: {
  status: LibraryStatus;
  catalogId?: string | null;
  navigationControl: ReactNode;
  jobsControl: ReactNode;
}) {
  const stopped = useCatalogSwitchState();
  return stopped ? <CatalogSwitchStopped state={stopped} /> : <SyncScreenContent {...props} />;
}

function SyncScreenContent({
  status,
  navigationControl,
  jobsControl,
}: {
  status: LibraryStatus;
  catalogId?: string | null;
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
  const [savedRuns, setSavedRuns] = useState<ComparisonRun[]>([]);
  useEffect(() => {
    let retired = false;
    void Promise.all([
      libraryData({ kind: 'backup_profiles' }, 'backup_profiles'),
      libraryData({ kind: 'backup_comparison_history' }, 'backup_comparison_history'),
    ])
      .then(([value, history]) => {
        if (retired) return;
        if (!Array.isArray(value))
          throw new Error('Comparison profiles could not be read. Retry profiles.');
        if (!Array.isArray(history))
          throw new Error('Scan history could not be read. Retry profiles.');
        setSavedRuns(history);
        setProfiles(value);
        const latest = history[0];
        if (latest?.profileId && value.some((item) => item.id === latest.profileId))
          setSelectedId((current) => current || latest.profileId!);
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
  // Saved profiles retain their source even if the catalog base changes.
  return (
    <BackupWorkspace
      key={JSON.stringify([
        selectedId,
        base?.id,
        base?.displayPath,
        base?.availability,
        registered?.displayPath,
        rebindFrom?.id,
        savedRuns[0]?.id,
      ])}
      rebindFrom={rebindFrom}
      savedRuns={savedRuns}
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
        <chakra.section aria-label="Saved comparisons">
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Saved comparison </span>
              </Field.Label>
              <NativeSelect
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
              </NativeSelect>
            </label>
          </Field.Root>
          {profileLoading && <Text role="status">Loading saved comparisons…</Text>}
          {profileError && (
            <Text role="alert">
              {profileError}{' '}
              <Button
                disabled={controlsBusy}
                onClick={() => {
                  setProfileLoading(true);
                  setProfileRefresh((value) => value + 1);
                }}
              >
                Retry profiles
              </Button>
            </Text>
          )}
          {selectedId && !profile && !profileLoading && (
            <Text role="alert">
              The selected profile is unavailable. Choose another profile or explicitly start a new
              comparison.
            </Text>
          )}
          {profile && (
            <Button
              disabled={controlsBusy}
              onClick={() => {
                setRebindFrom(profile);
                setSelectedId('');
              }}
            >
              Review profile rebinding
            </Button>
          )}
          {profile && (
            <Text>
              Bound to root {profile.sourceRootId}. To change source or destination, choose New
              comparison and save a separate profile. Existing bindings are retained.
            </Text>
          )}
        </chakra.section>
      }
      navigationControl={navigationControl}
      jobsControl={jobsControl}
    />
  );
}

function BackupWorkspace({
  rebindFrom,
  savedRuns,
  base,
  currentSourcePath,
  profile,
  profileControls,
  onProfileSaved,
  onActivityChange,
  navigationControl,
  jobsControl,
}: {
  savedRuns: ComparisonRun[];
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
  const [backgroundRun, setBackgroundRun] = useState<ComparisonRun | null>(null);
  const [history, setHistory] = useState(savedRuns);
  const [historical, setHistorical] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const pinnedHistory = useRef(false);
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
  const running = backgroundRun?.state === 'comparing' || run?.state === 'comparing';
  const runId = backgroundRun?.id ?? run?.id;
  const profileId = profile?.id;
  useEffect(() => {
    onActivityChange(busy || running);
  }, [busy, running, onActivityChange]);
  useEffect(() => {
    let retired = false;
    const token = generation.current;
    void libraryData({ kind: 'backup_comparison_latest' }, 'backup_comparison_latest')
      .then(async (latest) => {
        if (retired || token !== generation.current) return;
        const matches = (value: ComparisonRun) =>
          value.sourceRootId === base?.id &&
          value.sourceDisplayPath === currentSourcePath &&
          value.profileId === profileId;
        const saved = savedRuns.find(matches);
        const current = latest && matches(latest) ? latest : null;
        if (saved) {
          setRun(saved);
          setHistorical(true);
          setMode(saved.mode);
        }
        if (current?.state === 'comparing') {
          setBackgroundRun(current);
          return;
        }
        const retained = current ?? saved;
        if (!retained) return;
        setRun(retained);
        setHistorical(!current);
        setMode(retained.mode);
        if (!profileId || base?.availability !== 'online') {
          setNotice(
            'Saved results are available. Reconnect the source and choose the destination to scan for changes.',
          );
          return;
        }
        const jobs = await libraryData(
          { kind: 'backup_sync', request: { kind: 'list' } },
          'backup_sync',
        );
        if (retired || token !== generation.current) return;
        if (jobs.kind !== 'jobs')
          throw new Error(
            'Safe-copy activity could not be checked. Scan again after reviewing Jobs.',
          );
        if (
          jobs.data.some((job) =>
            [
              'queued',
              'running',
              'paused',
              'cancel_requested',
              'needs_review',
              'needs_recovery',
              'failed',
              'canceled',
              'completed_partial',
            ].includes(job.state),
          )
        ) {
          setNotice(
            'Saved results are available. Review the existing transfer before scanning again.',
          );
          return;
        }
        const next = await libraryData(
          { kind: 'backup_profile_compare', profile_id: profileId, mode: retained.mode },
          'backup_comparison',
        );
        if (retired || token !== generation.current) return;
        if (!matches(next))
          throw new Error('Comparison identity changed. Review this profile again.');
        setBackgroundRun(next);
        setHistorical(true);
        setNotice(
          'Checking for changes in the background. Your saved comparison stays available; no files are copied.',
        );
      })
      .catch((cause: unknown) => {
        if (!retired && token === generation.current) {
          setNotice(null);
          setError(errorMessage(cause));
        }
      });
    return () => {
      retired = true;
    };
    // Draft changes do not restart reconciliation. Scope changes remount this workspace.
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
        if (next.state === 'comparing') {
          setBackgroundRun(next);
          timer = setTimeout(() => void poll(), 500);
        } else {
          setBackgroundRun(null);
          setHistory((previous) =>
            [next, ...previous.filter((item) => item.id !== next.id)].slice(0, 32),
          );
          setNotice(
            ['failed', 'canceled', 'incomplete'].includes(next.state)
              ? 'The latest scan is incomplete. Review its outcome in history; previous results remain available.'
              : 'Latest scan finished. Earlier versions remain available in history.',
          );
          if (!pinnedHistory.current && !['failed', 'canceled'].includes(next.state)) {
            setRun(next);
            setHistorical(false);
            setPage(null);
            setOffset(0);
            setSelected(null);
          }
        }
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
    if (!run || run.state === 'comparing' || run.state === 'failed' || run.state === 'canceled')
      return;
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
    pinnedHistory.current = false;
    setHistorical(false);
    setRun(null);
    setPage(null);
    setOffset(0);
    setCategory('');
    setSelected(null);
  }

  return (
    <>
      <PageHeader>
        <chakra.header className="toolbar">
          {navigationControl}
          <Heading as="h1" unstyled>
            Sync
          </Heading>
          <Box className="header-actions">
            {run && (
              <Button disabled={busy || running} onClick={() => setSetupOpen((value) => !value)}>
                Scan settings
              </Button>
            )}
            <UtilityHistory
              versions={history
                .filter((item) => item.sourceRootId === base?.id && item.profileId === profileId)
                .map((item) => ({
                  id: item.id,
                  date: item.startedAt,
                  state: item.state,
                  label: `${item.mode === 'verified' ? 'Verified' : 'Quick'} comparison · ${item.entries} entries`,
                }))}
              selectedId={run?.id}
              disabled={transferBusy}
              onSelect={(id) => {
                const version = history.find((item) => item.id === id);
                if (version) {
                  pinnedHistory.current = true;
                  setRun(version);
                  setHistorical(true);
                  setPage(null);
                  setSelected(null);
                  setOffset(0);
                }
              }}
            />
            {jobsControl}
          </Box>
        </chakra.header>
      </PageHeader>
      <Box className="backup-workspace">
        {!run && !running && (
          <UtilityWelcome
            title="Keep a second folder in step."
            description="Choose a destination and comparison type, then scan to see what needs copying. Scanning leaves both folders unchanged. Saved comparisons will appear here next time."
          />
        )}
        {profileControls}
        {notice && <Text role="status">{notice}</Text>}
        {backgroundRun && (
          <chakra.section aria-label="Background comparison">
            <Text role="status">{backgroundRun.message}</Text>
            <progress aria-label="Background comparison in progress" />
          </chakra.section>
        )}
        {run && (
          <Text>
            Showing scan from {formatScanDate(run.startedAt)}
            {historical ? ' · Historical version; scan again before reviewing a transfer.' : ''}
          </Text>
        )}
        <details open={!run || setupOpen}>
          <summary
            onClick={(event) => {
              event.preventDefault();
              setSetupOpen((value) => !value);
            }}
          >
            Comparison settings
          </summary>
          {rebindFrom && (
            <chakra.section aria-label="Profile rebinding review">
              <Heading as="h2" unstyled>
                Review the new source and destination
              </Heading>
              <Text className="backup-path">
                Previous source: {rebindFrom.sourceLabel} · {rebindFrom.sourceDisplayPath} ·{' '}
                {rebindFrom.sourceRootId}
              </Text>
              <Text className="backup-path">
                Previous destination: {rebindFrom.destinationDisplayPath}
              </Text>
              <Text>
                The cards below show the current base and newly selected destination. The previous
                profile and historical observations are retained. A fresh comparison is required; no
                destination content is adopted or removed.
              </Text>
              <label>
                <input
                  type="checkbox"
                  checked={rebindReviewed}
                  onChange={(event) => setRebindReviewed(event.target.checked)}
                />
                I reviewed the previous and new source/destination bindings
              </label>
            </chakra.section>
          )}
          <Heading as="h2" unstyled>
            Compare before copying
          </Heading>
          <Text>
            Library → Destination. Comparison leaves both folders unchanged. Review transfer
            capabilities, proposed actions and any limits below before approving a copy.
          </Text>
          <Box className="backup-scope">
            <chakra.section aria-label="Fixed source library">
              <Heading as="h3" unstyled>
                Library (source)
              </Heading>
              <strong>{base?.label ?? 'No base library configured'}</strong>
              <Text className="backup-path">{base?.displayPath}</Text>
              <Text>
                {base?.availability ?? 'unavailable'} · All regular files, including hidden files,
                sidecars and unknown formats.
              </Text>
            </chakra.section>
            <chakra.section aria-label="Comparison destination">
              <Heading as="h3" unstyled>
                Destination
              </Heading>
              <Text className="backup-path">
                {profile?.destinationDisplayPath ??
                  destination?.displayPath ??
                  'Choose a separate folder.'}
              </Text>
              <Button
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
              </Button>
              <Text>
                Only the proposed Stillharbour dataset is compared. Other files in this folder
                remain outside the scope. Existing data is not adopted by selecting it.
              </Text>
            </chakra.section>
          </Box>
          {!profile && (
            <Box className="backup-controls">
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Comparison profile name </span>
                  </Field.Label>
                  <Input
                    value={profileLabel}
                    maxLength={80}
                    disabled={busy || running}
                    onChange={(event) => setProfileLabel(event.target.value)}
                  />
                </label>
              </Field.Root>
              <Button
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
              </Button>
              <Text>
                Remembers this source and destination for future comparisons. Saving a profile does
                not approve copying or create destination files.
              </Text>
            </Box>
          )}
          <Box className="backup-controls">
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Comparison </span>
                </Field.Label>
                <NativeSelect
                  value={mode}
                  disabled={busy || running}
                  onChange={(event) => {
                    retire();
                    setMode(event.target.value as CompareMode);
                  }}
                >
                  <option value="verified">Verified (complete bytes)</option>
                  <option value="quick">Quick (content not checked)</option>
                </NativeSelect>
              </label>
            </Field.Root>
            <Button
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
                    throw new Error('The base library changed. Reopen Sync and compare again.');
                  setRun(result);
                  setBackgroundRun(result);
                  setSetupOpen(false);
                })
              }
            >
              Compare
            </Button>
            {running && (
              <Button
                disabled={busy}
                onClick={() =>
                  void action(async () => {
                    const current = backgroundRun ?? run;
                    if (!current) return;
                    await libraryData(
                      { kind: 'backup_compare_cancel', run_id: current.id },
                      'backup_comparison',
                    );
                  })
                }
              >
                Cancel comparison
              </Button>
            )}
          </Box>
        </details>
        {running && run && (
          <Button
            disabled={busy}
            onClick={() =>
              void action(async () => {
                await libraryData(
                  { kind: 'backup_compare_cancel', run_id: backgroundRun?.id ?? run.id },
                  'backup_comparison',
                );
              })
            }
          >
            Stop background scan
          </Button>
        )}
        <Text>
          Comparison is not a backup or an approval. Preparation, copying and recovery each require
          a separate action review.
        </Text>
        {!base && <Text role="status">Configure a base library before comparing.</Text>}
        {base && base.availability !== 'online' && (
          <Text role="status">
            The source is unavailable. It is not an empty library; reconnect it before comparing.
          </Text>
        )}
        {destination && !destination.identityAvailable && (
          <Text role="alert">
            This destination cannot supply the required native identity. Choose a different folder.
          </Text>
        )}
        {error && (
          <Text className="error" role="alert">
            {error}
          </Text>
        )}
        {run && (
          <chakra.section aria-label="Comparison review">
            <Heading as="h3" unstyled>
              {run.state.replaceAll('_', ' ')}
            </Heading>
            <Text role="status">{run.message}</Text>
            <Text className="backup-path">Dataset: {run.datasetDisplayPath}</Text>
            <Text>
              {run.historyMessage} Prior observation:{' '}
              {run.baselineId ?? 'none (initial comparison)'}
            </Text>
            <Text>
              {run.entries} entries observed · {run.bytesRead} content bytes read
            </Text>
            <Text>
              Modified timestamps are filesystem observations in UTC, with filesystem-dependent
              precision; they are separate from capture and birth dates.
            </Text>
            {run.summary && (
              <List.Root unstyled aria-label="Comparison totals" className="backup-totals">
                {categories.map((finding) => (
                  <List.Item unstyled key={finding}>
                    {finding.replaceAll('_', ' ')}: {run.summary![summaryKeys[finding]]}
                  </List.Item>
                ))}
              </List.Root>
            )}
            <Text>
              Reads use the shared application I/O budget. Filesystem metadata, other applications
              and physical disk traffic are outside that byte cap. Reviews are session-only and
              expire on restart or a new comparison.
            </Text>
            {run.issues.length > 0 && (
              <details open>
                <summary>Issues ({run.issues.length}; first 200 shown)</summary>
                <List.Root unstyled>
                  {run.issues.map((issue, index) => (
                    <List.Item unstyled key={index}>
                      {issue.side && `${issue.side}: `}
                      <Text as="span" className="backup-path">
                        {issue.displayPath}
                      </Text>
                      : {issue.message}
                    </List.Item>
                  ))}
                </List.Root>
              </details>
            )}
            {run.state !== 'comparing' && run.state !== 'failed' && run.state !== 'canceled' && (
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Finding filter </span>
                  </Field.Label>
                  <NativeSelect
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
                  </NativeSelect>
                </label>
              </Field.Root>
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
                  <chakra.section aria-label="Entry details" className="backup-entry-details">
                    <Heading as="h4" unstyled className="backup-path">
                      {selected.displayPath}
                    </Heading>
                    <Text>
                      {selected.category.replaceAll('_', ' ')} ·{' '}
                      {selected.proposal.replaceAll('_', ' ')}
                    </Text>
                    <Text>{selected.evidence}</Text>
                    <Text>
                      Potential same-stem companions (up to 32; each retains its own evidence):{' '}
                      {selected.companions?.join(' · ') || 'none displayed'}. Name association does
                      not prove bundle completeness or redundancy.
                    </Text>
                    <dl>
                      {(['source', 'destination'] as const).map((side) => (
                        <Box key={side}>
                          <dt>{side === 'source' ? 'Source' : 'Destination'}</dt>
                          <dd>
                            {selected[`${side}Kind`] ??
                              'Not observed; absence is established only by a complete inventory'}{' '}
                            · {selected[`${side}Bytes`] ?? 'unknown'} bytes ·{' '}
                            {selected[`${side}Modified`] ?? 'unknown modified time'}
                          </dd>
                        </Box>
                      ))}
                    </dl>
                    <Text>
                      Observed metadata and comparison evidence only. This review does not establish
                      companion completeness, metadata fidelity or recovery.
                    </Text>
                    {!historical && !running && (
                      <ComparisonPreviews
                        key={`${run.id}:${selected.id}`}
                        runId={run.id}
                        entryId={selected.id}
                      />
                    )}
                    <Button onClick={() => setSelected(null)}>Close entry details</Button>
                  </chakra.section>
                )}
                <Box className="backup-controls">
                  <Button
                    disabled={offset === 0}
                    onClick={() => {
                      setPage(null);
                      setSelected(null);
                      setOffset(Math.max(0, offset - 200));
                    }}
                  >
                    Previous
                  </Button>
                  <Text as="span">
                    Entries {page.entries.length ? offset + 1 : 0}–{offset + page.entries.length}
                    {page.total !== undefined && ` of ${page.total}`}
                  </Text>
                  <Button
                    disabled={page.nextOffset === null}
                    onClick={() => {
                      setPage(null);
                      setSelected(null);
                      setOffset(page.nextOffset ?? 0);
                    }}
                  >
                    Next
                  </Button>
                </Box>
              </>
            )}
          </chakra.section>
        )}
        <SyncWorkflow
          key={historical || running ? 'no-comparison' : (run?.id ?? 'no-comparison')}
          run={historical || running ? null : run}
          selected={selected}
          onActivityChange={setTransferBusy}
        />
      </Box>
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
    <Box className="backup-table">
      <Table.Root unstyled>
        <Table.Caption unstyled captionSide="top">
          Comparison findings and proposed actions
        </Table.Caption>
        <Table.Header unstyled>
          <Table.Row unstyled>
            <Table.ColumnHeader unstyled>Relative path</Table.ColumnHeader>
            <Table.ColumnHeader unstyled>Source bytes / modified (UTC)</Table.ColumnHeader>
            <Table.ColumnHeader unstyled>Destination bytes / modified (UTC)</Table.ColumnHeader>
            <Table.ColumnHeader unstyled>Finding</Table.ColumnHeader>
            <Table.ColumnHeader unstyled>Proposed action / evidence</Table.ColumnHeader>
          </Table.Row>
        </Table.Header>
        <Table.Body unstyled>
          {entries.map((entry) => (
            <Table.Row unstyled key={entry.id}>
              <Table.Cell unstyled className="backup-path">
                <Button
                  disabled={disabled}
                  aria-pressed={selectedId === entry.id}
                  onClick={() => onSelect(entry)}
                >
                  {entry.displayPath}
                </Button>
              </Table.Cell>
              <Table.Cell unstyled>
                {entry.sourceBytes ?? '—'}
                <br />
                {entry.sourceModified ?? 'unknown'}
              </Table.Cell>
              <Table.Cell unstyled>
                {entry.destinationBytes ?? '—'}
                <br />
                {entry.destinationModified ?? 'unknown'}
              </Table.Cell>
              <Table.Cell unstyled>{entry.category.replaceAll('_', ' ')}</Table.Cell>
              <Table.Cell unstyled>
                {entry.proposal.replaceAll('_', ' ')}
                <Text>{entry.evidence}</Text>
              </Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table.Root>
    </Box>
  );
}
