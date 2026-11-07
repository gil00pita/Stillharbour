import { PageHeader } from '@/components/PageHeader';
import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Box } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { DuplicateMatchOptions } from '@/components/DuplicateMatchOptions';
import { InspectionChoiceCards } from '@/components/InspectionChoiceCards';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import type {
  HealthMode,
  InspectionInput,
  DuplicateOptions,
  DuplicatePage,
  FindingPage,
  InspectionJob,
  InspectionReport,
  LibraryStatus,
  PropertyRules,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { StorageExplorer } from '@/components/StorageExplorer';
import { InspectionDialog } from '@/components/InspectionDialog';
import { DuplicateStudio } from '@/components/DuplicateStudio';
import { IntegrityFindings } from '@/components/IntegrityFindings';
import { InspectionIssues } from '@/components/InspectionIssues';
import { displayBytes } from '../../inspection/format';
import { UtilityWelcome } from '@/components/UtilityWelcome';
import { UtilityHistory } from '@/components/UtilityHistory';
import { formatScanDate } from '../../utils/format-scan-date';

export type InspectionSection = 'Duplicates' | 'Integrity' | 'Storage';
const fileScopeChoices = [
  {
    value: 'indexed_media_companions',
    label: 'Photos & companions',
    description: 'Indexed photos, videos and matching companion files.',
  },
  {
    value: 'all_eligible_files',
    label: 'All eligible files',
    description: 'Every eligible file within the exclusions below.',
  },
] as const;
const initialRules: PropertyRules = {
  logic: 'all',
  filename: true,
  extension: false,
  size: false,
  modified: false,
  created: false,
  readonly: false,
  content: false,
  toleranceMillis: 0,
  name: { lowercase: false, ignoreWhitespace: false, ignoredCharacters: '', unicodeNfc: false },
};
function active(job: InspectionJob | null) {
  return job !== null && ['queued', 'running', 'cancel_requested'].includes(job.state);
}
function taskMatches(job: InspectionJob, section: InspectionSection) {
  return (
    job.input.task.kind ===
    (section === 'Duplicates' ? 'duplicates' : section === 'Integrity' ? 'health' : 'storage')
  );
}

export function InspectionScreen({
  section,
  status,
  navigationControl,
  jobsControl,
}: {
  section: InspectionSection;
  status: LibraryStatus;
  navigationControl: ReactNode;
  jobsControl: ReactNode;
}) {
  const [rootIds, setRootIds] = useState(() => status.roots.map((root) => root.id));
  const [scope, setScope] = useState<'indexed_media_companions' | 'all_eligible_files'>(
    section === 'Storage' ? 'all_eligible_files' : 'indexed_media_companions',
  );
  const [options, setOptions] = useState<DuplicateOptions>({
    mode: 'exact_content',
    algorithm: 'blake3',
    rules: initialRules,
    excludeHardlinkedObjects: false,
  });
  const [healthMode, setHealthMode] = useState<HealthMode>('compare_baseline');
  const [baselineAcknowledged, setBaselineAcknowledged] = useState(false);
  const [job, setJob] = useState<InspectionJob | null>(null);
  const [history, setHistory] = useState<InspectionJob[]>([]);
  const [report, setReport] = useState<InspectionReport | null>(null);
  const [files, setFiles] = useState<FindingPage | null>(null);
  const [groups, setGroups] = useState<DuplicatePage | null>(null);
  const [viewedJob, setViewedJob] = useState<InspectionJob | null>(null);
  const [loading, setLoading] = useState(true);
  const [setupOpen, setSetupOpen] = useState(false);
  const [storageDetailsOpen, setStorageDetailsOpen] = useState(false);
  const storageDetailsOpener = useRef<HTMLButtonElement>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const pinnedHistory = useRef(false);
  const [historical, setHistorical] = useState(false);
  const resultEpoch = useRef(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [failedResults, setFailedResults] = useState<InspectionJob | null>(null);
  const [reload, setReload] = useState(0);
  const epoch = useRef(0);
  const running = active(job);
  const jobId = job?.id ?? null;
  const loadResults = useCallback(async (record: InspectionJob, token: number) => {
    try {
      const [summary, rows, matches] = await Promise.all([
        libraryData({ kind: 'inspection_report', job_id: record.id }, 'inspection_report'),
        libraryData(
          { kind: 'inspection_findings', job_id: record.id, after: null },
          'inspection_findings',
        ),
        record.input.task.kind === 'duplicates'
          ? libraryData(
              { kind: 'inspection_groups', job_id: record.id, after: null },
              'inspection_groups',
            )
          : Promise.resolve(null),
      ]);
      if (token !== resultEpoch.current) return;
      setViewedJob(record);
      setReport(summary);
      setFiles(rows);
      setGroups(matches);
      setFailedResults(null);
    } catch (failure) {
      if (token === resultEpoch.current) setFailedResults(record);
      throw failure;
    }
  }, []);
  useEffect(() => {
    let disposed = false;
    void libraryData({ kind: 'inspection_jobs' }, 'inspection_jobs')
      .then(async (jobs) => {
        if (disposed) return;
        const matching = jobs.filter((record) => taskMatches(record, section));
        setHistory(matching);
        const latest = matching[0];
        if (!latest) return;
        setJob(latest);
        setRootIds(latest.input.rootIds);
        setScope(latest.input.scope);
        if (latest.input.task.kind === 'duplicates') setOptions(latest.input.task.options);
        if (latest.input.task.kind === 'health')
          setHealthMode(
            latest.input.task.mode === 'establish_baseline'
              ? 'compare_baseline'
              : latest.input.task.mode,
          );
        const retained =
          matching.find((record) => ['completed', 'partial'].includes(record.state)) ??
          matching.find((record) => !active(record));
        if (retained) await loadResults(retained, ++resultEpoch.current);
        if (disposed || active(latest)) return;
        if (latest.state === 'failed' || latest.state === 'canceled') return;
        if (jobs.some(active)) {
          setNotice(
            'Another inspection is running. Saved results remain available; scan again when it finishes.',
          );
          return;
        }
        if (
          !latest.input.rootIds.every((id) =>
            status.roots.some((root) => root.id === id && root.availability === 'online'),
          )
        ) {
          setNotice(
            'Saved results are available. Reconnect the selected libraries to check for changes.',
          );
          return;
        }
        // Reconciliation is a new read-only run. Never replay baseline establishment.
        const input: InspectionInput =
          latest.input.task.kind === 'health' && latest.input.task.mode === 'establish_baseline'
            ? { ...latest.input, task: { kind: 'health', mode: 'compare_baseline' } }
            : latest.input;
        setNotice('Checking for changes in the background. Your saved results stay available.');
        const next = await libraryData({ kind: 'inspection_start', input }, 'inspection_job');
        if (!disposed) {
          setJob(next);
          setHistory((previous) =>
            [next, ...previous.filter((record) => record.id !== next.id)].slice(0, 64),
          );
        }
      })
      .catch((failure) => {
        if (!disposed) {
          setNotice(null);
          setError(errorMessage(failure));
        }
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
      resultEpoch.current += 1;
    };
    // Opening a utility restores its scope once; changing draft settings must not restart a scan.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [section, loadResults, reload]);
  useEffect(() => {
    if (!jobId || !running) return;
    const id = jobId;
    const token = epoch.current;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const next = await libraryData({ kind: 'inspection_job', job_id: id }, 'inspection_job');
        if (stopped || token !== epoch.current) return;
        setJob(next);
        setHistory((previous) =>
          [next, ...previous.filter((record) => record.id !== next.id)].slice(0, 64),
        );
        if (!active(next)) {
          if (!pinnedHistory.current && ['completed', 'partial'].includes(next.state))
            await loadResults(next, ++resultEpoch.current);
          setNotice(
            next.state === 'completed'
              ? 'Latest scan finished. Earlier versions remain available in history.'
              : 'Scan stopped with an incomplete outcome. Review its status and coverage.',
          );
          return;
        }
      } catch (failure) {
        if (!stopped && token === epoch.current) setError(errorMessage(failure));
      }
      if (!stopped) timer = setTimeout(() => void poll(), 600);
    }
    timer = setTimeout(() => void poll(), 200);
    return () => {
      stopped = true;
      clearTimeout(timer);
    };
  }, [jobId, running, loadResults]);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  async function start() {
    const token = ++epoch.current;
    setBusy(true);
    setError(null);
    setFailedResults(null);
    pinnedHistory.current = false;
    setHistorical(false);
    setNotice(null);
    try {
      const next = await libraryData(
        {
          kind: 'inspection_start',
          input: {
            rootIds,
            scope,
            task:
              section === 'Duplicates'
                ? { kind: 'duplicates', options }
                : section === 'Integrity'
                  ? { kind: 'health', mode: healthMode }
                  : { kind: 'storage' },
          },
        },
        'inspection_job',
      );
      if (token === epoch.current) {
        setJob(next);
        setSetupOpen(false);
        setHistory((previous) => [next, ...previous].slice(0, 64));
      }
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
    } finally {
      if (token === epoch.current) setBusy(false);
    }
  }
  async function retryResults() {
    if (!failedResults) return;
    setBusy(true);
    setError(null);
    try {
      await loadResults(failedResults, ++resultEpoch.current);
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setBusy(false);
    }
  }
  async function selectJob(record: InspectionJob) {
    if (active(record)) {
      pinnedHistory.current = false;
      return;
    }
    pinnedHistory.current = true;
    setHistorical(true);
    setError(null);
    try {
      await loadResults(record, ++resultEpoch.current);
    } catch (failure) {
      setError(errorMessage(failure));
    }
  }
  async function stop() {
    if (!job) return;
    const token = epoch.current;
    setBusy(true);
    try {
      const next = await libraryData(
        { kind: 'inspection_cancel', job_id: job.id },
        'inspection_job',
      );
      if (token === epoch.current) setJob(next);
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
    } finally {
      if (token === epoch.current) setBusy(false);
    }
  }
  async function nextFiles() {
    if (!viewedJob || !files?.next) return;
    const token = resultEpoch.current;
    try {
      const page = await libraryData(
        { kind: 'inspection_findings', job_id: viewedJob.id, after: files.next },
        'inspection_findings',
      );
      if (token === resultEpoch.current) setFiles(page);
    } catch (failure) {
      if (token === resultEpoch.current) setError(errorMessage(failure));
    }
  }
  async function nextGroups() {
    if (!viewedJob || !groups?.next) return;
    const token = resultEpoch.current;
    try {
      const page = await libraryData(
        { kind: 'inspection_groups', job_id: viewedJob.id, after: groups.next },
        'inspection_groups',
      );
      if (token === resultEpoch.current) setGroups(page);
    } catch (failure) {
      if (token === resultEpoch.current) setError(errorMessage(failure));
    }
  }
  const unavailable = rootIds.some(
    (id) => status.roots.find((root) => root.id === id)?.availability !== 'online',
  );
  const invalidRules =
    options.mode === 'properties' &&
    ![
      options.rules.filename,
      options.rules.extension,
      options.rules.size,
      options.rules.modified,
      options.rules.created,
      options.rules.readonly,
      options.rules.content,
    ].some(Boolean);
  const inspectionDetails = (
    <>
      {' '}
      <chakra.details
        className="inspection-setup"
        css={{
          marginBlock: '24px',
          '& > summary': {
            cursor: 'pointer',
            color: '{colors.fg.secondary}',
            fontSize: '13px',
            paddingBlock: '12px',
            borderBlock: '1px solid {colors.border}',
          },
        }}
        open={history.length === 0 || setupOpen}
      >
        <summary
          onClick={(event) => {
            event.preventDefault();
            setSetupOpen((value) => !value);
          }}
        >
          {section === 'Duplicates'
            ? 'Duplicate inspection settings'
            : section === 'Integrity'
              ? 'Health check settings'
              : 'Storage measurement settings'}
        </summary>
        <chakra.fieldset
          disabled={busy || running}
          className={`inspection-controls ${section === 'Duplicates' ? 'duplicate-controls' : ''}`}
          css={{
            '& > .primary': { marginTop: '16px', gridColumn: '1 / -1', justifySelf: 'start' },
            '& > p': { maxWidth: '80ch', fontSize: '13px', gridColumn: '1 / -1' },
            border: '1px solid {colors.border}',
            borderRadius: '10px',
            padding: '16px',
            margin: '16px 0',
            background: '{colors.bg.panel}',
            '& legend': { padding: '0 6px' },
            '& fieldset': {
              border: '1px solid {colors.border}',
              margin: '12px 0',
              padding: '12px',
            },
            display: 'grid',
            gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
            gap: '0 24px',
            '& > legend': { gridColumn: '1 / -1' },
            '& > .inspection-roots': { gridColumn: '1 / -1' },
            '& > fieldset': { gridColumn: '1 / -1' },
            '@media (max-width: 760px)': { display: 'block' },
            '&.duplicate-controls': {
              padding: '0',
              border: '0',
              background: 'transparent',
              margin: '24px 0',
              '& .inspection-roots': { margin: '20px 0' },
            },
            '& > .duplicate-match-options': { gridColumn: '1 / -1' },
            '& > .inspection-choice-group': { gridColumn: '1 / -1' },
          }}
        >
          <chakra.legend
            className={section === 'Duplicates' ? 'screen-reader-status' : undefined}
            css={{
              '&.screen-reader-status': {
                position: 'absolute',
                width: '1px',
                height: '1px',
                overflow: 'hidden',
                clipPath: 'inset(50%)',
              },
            }}
          >
            {section === 'Duplicates'
              ? 'Find duplicate clusters'
              : section === 'Integrity'
                ? 'Check file health'
                : 'Measure storage usage'}
          </chakra.legend>
          {section === 'Duplicates' && (
            <DuplicateMatchOptions
              options={options}
              onChange={setOptions}
              disabled={busy || running}
            />
          )}
          <Box
            className="inspection-roots"
            css={{
              display: 'flex',
              flexWrap: 'wrap',
              gap: '8px 20px',
              alignItems: 'center',
              '& small': { color: '{colors.fg.muted}' },
            }}
          >
            {rootIds
              .filter((id) => !status.roots.some((root) => root.id === id))
              .map((id) => (
                <label key={id}>
                  <input
                    type="checkbox"
                    checked
                    onChange={() => setRootIds((ids) => ids.filter((value) => value !== id))}
                  />
                  Previously registered folder {id.slice(0, 8)}
                  <small>Unavailable · exclude from the next scan to continue</small>
                </label>
              ))}
            {status.roots.map((root) => (
              <label key={root.id}>
                <input
                  type="checkbox"
                  checked={rootIds.includes(root.id)}
                  onChange={(event) =>
                    setRootIds(
                      event.target.checked
                        ? [...rootIds, root.id]
                        : rootIds.filter((id) => id !== root.id),
                    )
                  }
                />
                {root.label}
                <small>{root.availability}</small>
              </label>
            ))}
          </Box>
          {section === 'Duplicates' ? (
            <InspectionChoiceCards
              label="File scope"
              value={scope}
              choices={fileScopeChoices}
              disabled={busy || running}
              onChange={setScope}
            />
          ) : (
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>File scope</span>
                </Field.Label>
                <NativeSelect
                  value={scope}
                  onChange={(event) => setScope(event.target.value as typeof scope)}
                >
                  <option value="indexed_media_companions">
                    Indexed photos, videos and matching companion files
                  </option>
                  <option value="all_eligible_files">All files within the exclusions below</option>
                </NativeSelect>
              </label>
            </Field.Root>
          )}
          <Text>
            Your hidden-file, hidden-folder, filename-pattern and size exclusions still apply.
            Gallery file-type and date filters do not limit this inspection. Links, special files
            and cloud placeholders are reported, never followed or downloaded.
          </Text>
          {section === 'Duplicates' && (
            <>
              <Text>
                Exact matches are confirmed by comparing every byte. Matching names, sizes or other
                properties only identify candidates; they never authorize quarantine.
              </Text>
              <Box>
                {options.mode === 'properties' && (
                  <PropertyControls
                    rules={options.rules}
                    disabled={busy || running}
                    onChange={(rules) => setOptions({ ...options, rules })}
                  />
                )}
              </Box>
            </>
          )}
          {unavailable && (
            <Text role="status">
              Reconnect selected unavailable folders before inspecting. Their retained inventory is
              not an empty folder.
            </Text>
          )}
          {invalidRules && <Text role="status">Choose at least one property criterion.</Text>}
          {section === 'Integrity' && (
            <>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Integrity mode</span>
                  </Field.Label>
                  <NativeSelect
                    aria-label="Integrity mode"
                    value={healthMode}
                    onChange={(event) => {
                      setHealthMode(event.target.value as HealthMode);
                      setBaselineAcknowledged(false);
                    }}
                  >
                    <option value="compare_baseline">Compare with saved checksums</option>
                    <option value="establish_baseline">Record new checksum baselines</option>
                    <option value="quick">Quick signature and header check</option>
                    <option value="deep">Decode supported still images</option>
                  </NativeSelect>
                </label>
              </Field.Root>
              <Text>
                Checksums read complete file bytes. A mismatch means changed since baseline, not
                proven damage. Previous baselines remain in history; these checks do not establish
                backup or original-file recovery.
              </Text>
              {healthMode === 'establish_baseline' && (
                <label>
                  <input
                    type="checkbox"
                    checked={baselineAcknowledged}
                    onChange={(event) => setBaselineAcknowledged(event.target.checked)}
                  />
                  Record newly read baselines and retain every previous baseline.
                </label>
              )}
            </>
          )}
          {
            <Button
              className="primary"
              disabled={
                loading ||
                !rootIds.length ||
                unavailable ||
                invalidRules ||
                (section === 'Integrity' &&
                  healthMode === 'establish_baseline' &&
                  !baselineAcknowledged)
              }
              onClick={() => void start()}
            >
              {section === 'Integrity'
                ? healthMode === 'establish_baseline'
                  ? 'Establish checksum baselines'
                  : healthMode === 'quick'
                    ? 'Start quick header check'
                    : healthMode === 'deep'
                      ? 'Start deep image check'
                      : 'Start checksum comparison'
                : `Start ${section === 'Duplicates' ? 'duplicate' : 'storage'} inspection`}
            </Button>
          }
        </chakra.fieldset>
      </chakra.details>
      {job && (
        <chakra.section
          className="inspection-job"
          css={{
            border: '1px solid {colors.border}',
            borderRadius: '10px',
            padding: '16px',
            margin: '16px 0',
            background: '{colors.bg.panel}',
            '& progress': { display: 'block', width: 'min(360px, 100%)', margin: '12px 0' },
          }}
          aria-label="Inspection job"
        >
          <Heading as="h2" unstyled>
            {job.state.replaceAll('_', ' ')}
          </Heading>
          <Text role="status">{job.message}</Text>
          <Text>
            {job.filesObserved.toLocaleString()} observed · {job.filesChecked.toLocaleString()}{' '}
            examined · {displayBytes(job.bytesRead)} read · {job.issues.toLocaleString()} issues
          </Text>
          {running && (
            <>
              <progress aria-label="Inspection in progress" />
              <Button
                disabled={busy || job.state === 'cancel_requested'}
                onClick={() => void stop()}
              >
                Cancel inspection
              </Button>
            </>
          )}
          {!running && (job.state === 'failed' || job.state === 'canceled') && (
            <Button
              disabled={busy || loading || rootIds.length === 0 || unavailable || invalidRules}
              onClick={() => {
                if (section === 'Integrity' && healthMode === 'establish_baseline') {
                  setBaselineAcknowledged(false);
                  setSetupOpen(true);
                } else {
                  void start();
                }
              }}
            >
              {section === 'Integrity' ? 'Retry integrity check' : 'Retry inspection'}
            </Button>
          )}
          {job.limited && (
            <Text>
              Inspection reached a bound. These results do not cover the whole selected scope.
            </Text>
          )}
          <Text>
            Retained scope: {job.input.scope.replaceAll('_', ' ')} ·{' '}
            {job.input.rootIds
              .map(
                (id) =>
                  status.roots.find((root) => root.id === id)?.label ??
                  `Previously registered folder ${id.slice(0, 8)}`,
              )
              .join(', ')}
            . Re-run to check current files; retained results grant no mutation approval.
          </Text>
        </chakra.section>
      )}
      {report && (
        <>
          {viewedJob && <InspectionIssues key={viewedJob.id} jobId={viewedJob.id} />}
          {section === 'Duplicates' && (
            <details>
              <summary>Format applicability</summary>
              <Text>{report.id3Applicability}</Text>
            </details>
          )}
          <Text className="inspection-coverage">
            Inventory:{' '}
            {report.coverage.enumerationComplete
              ? 'completed for this eligible scope'
              : 'incomplete'}{' '}
            · {report.coverage.excludedEntries} excluded entries/scopes ·{' '}
            {report.coverage.inaccessibleEntries} inaccessible/unsupported entries ·{' '}
            {report.coverage.incompleteDirectories} incomplete directories.
          </Text>
        </>
      )}
    </>
  );
  const versionLabel = viewedJob && (
    <Text>
      Showing scan from {formatScanDate(viewedJob.startedAt)} ·{' '}
      {viewedJob.state.replaceAll('_', ' ')}
      {historical ? ' · Historical version' : ''}
      {section !== 'Storage' && (
        <>
          <br />
          {viewedJob.filesChecked.toLocaleString()} files examined ·{' '}
          {viewedJob.input.scope.replaceAll('_', ' ')} ·{' '}
          {viewedJob.input.rootIds
            .map(
              (id) =>
                status.roots.find((root) => root.id === id)?.label ??
                `Previously registered folder ${id.slice(0, 8)}`,
            )
            .join(', ')}
        </>
      )}
    </Text>
  );
  return (
    <Box
      className={`inspection-page inspection-section-${section.toLowerCase()}`}
      css={{ display: 'flex', flexDirection: 'column', height: '100%', minHeight: '0' }}
    >
      <PageHeader>
        <chakra.header
          className="toolbar"
          css={{
            flexShrink: '0',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '16px',
            minHeight: '64px',
            padding: '12px 24px',
            borderBottom: '1px solid {colors.border}',
            background: '{colors.bg.toolbar}',
            '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
          }}
        >
          {navigationControl}
          <Box minW="0">
            <Heading as="h1" unstyled>
              {section}
            </Heading>
          </Box>
          <Box
            className="header-actions"
            css={{ display: 'flex', alignItems: 'center', gap: '8px' }}
          >
            {section === 'Storage' ? (
              <>
                <Button
                  ref={storageDetailsOpener}
                  aria-label="Storage measurement"
                  aria-haspopup="dialog"
                  onClick={() => setStorageDetailsOpen(true)}
                >
                  {viewedJob || job ? 'Measurement details' : 'Measure storage'}
                </Button>
                {(job?.limited || (report && !report.coverage.enumerationComplete)) && (
                  <small role="status">Incomplete measurement</small>
                )}
              </>
            ) : (
              history.length > 0 && (
                <Button disabled={busy || running} onClick={() => setSetupOpen((value) => !value)}>
                  Scan settings
                </Button>
              )
            )}
            <UtilityHistory
              versions={history.map((record) => ({
                id: record.id,
                date: record.startedAt,
                state: record.state,
                label:
                  record.input.task.kind === 'duplicates'
                    ? record.input.task.options.mode.replaceAll('_', ' ')
                    : record.input.task.kind === 'health'
                      ? record.input.task.mode.replaceAll('_', ' ')
                      : 'Storage usage',
              }))}
              selectedId={viewedJob?.id}
              onSelect={(id) => {
                const record = history.find((item) => item.id === id);
                if (record) void selectJob(record);
              }}
            />
            {jobsControl}
          </Box>
        </chakra.header>
      </PageHeader>
      <Box
        className="inspection-content"
        css={{
          overflow: 'auto',
          minHeight: '0',
          padding: '24px',
          '& > *': { maxWidth: '1440px' },
          '& h2:not(:where(.duplicate-studio *, .storage-explorer *))': {
            fontSize: '18px',
            margin: '24px 0 12px',
          },
          '& h3:not(:where(.duplicate-studio *, .storage-explorer *))': { fontSize: '16px' },
          '& p:not(:where(.duplicate-studio *, .storage-explorer *))': {
            color: '{colors.fg.muted}',
          },
          '&\n  label:not(:where(.duplicate-studio *, .storage-explorer *)):not(.inspection-choice-card):not(\n    .inspection-choice-label\n  )':
            {
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'stretch',
              gap: '8px',
              margin: '12px 0',
              fontSize: '13px',
            },
          "&\n  label:not(:where(.duplicate-studio *, .storage-explorer *)):has(> input[type='checkbox'])":
            { flexDirection: 'row', alignItems: 'center' },
          "&\n  input:not([type='checkbox']):not([type='radio']):not(\n    :where(.duplicate-studio *, .storage-explorer *)\n  )":
            {
              padding: '7px 10px',
              maxWidth: '100%',
              color: 'inherit',
              background: '{colors.bg}',
              border: '1px solid {colors.border}',
              borderRadius: 'var(--control-radius)',
            },
          '& select:not(:where(.duplicate-studio *, .storage-explorer *))': {
            padding: '7px 10px',
            maxWidth: '100%',
            color: 'inherit',
            background: '{colors.bg}',
            border: '1px solid {colors.border}',
            borderRadius: 'var(--control-radius)',
          },
          '@media (max-width: 800px)': { padding: '16px' },
          '& :is(button, input, summary):focus-visible': {
            outline: '2px solid {colors.brand.solid}',
            outlineOffset: '3px',
          },
          "& input[type='checkbox']": { accentColor: '{colors.brand.solid}' },
          '& ::selection': { background: '{colors.brand.solid}', color: '{colors.bg}' },
          '@media (max-width: 760px)': { '&:has(.duplicate-controls)': { padding: '20px 16px' } },
        }}
      >
        {section === 'Storage' && (
          <>
            {notice && <Text role="status">{notice}</Text>}
            {versionLabel}
          </>
        )}
        {!loading && !history.length && (
          <UtilityWelcome
            title={
              section === 'Duplicates'
                ? 'Find the duplicates worth reviewing.'
                : section === 'Integrity'
                  ? 'Get to know the health of your library.'
                  : 'See where your storage goes.'
            }
            description="Choose your libraries and scan settings below. Your first results will appear here, and each scan keeps its own history. Originals stay unchanged."
          />
        )}
        {loading && <Text role="status">Loading saved scans…</Text>}
        {section !== 'Storage' && (
          <>
            {notice && <Text role="status">{notice}</Text>}
            {versionLabel}
            <Text>
              {section === 'Duplicates'
                ? 'Find identical files or compare file properties. Review the matches while your originals stay unchanged.'
                : 'Check integrity without rewriting or repairing originals.'}
            </Text>
          </>
        )}
        {error && (section !== 'Storage' || !storageDetailsOpen) && (
          <Box
            role="alert"
            className="error"
            css={{
              border: '1px solid {colors.border.control}',
              borderRadius: 'var(--control-radius)',
              background: '{colors.danger.bg}',
              color: '{colors.danger.fg}',
              padding: '16px 20px',
              margin: '24px 0 0',
              fontSize: '13px',
              lineHeight: '1.6',
              '& p': { margin: '0' },
              '& button': {
                marginTop: '12px',
                border: '1px solid currentColor',
                background: 'transparent',
                color: 'inherit',
                borderRadius: '4px',
                padding: '7px 12px',
              },
            }}
          >
            <Text>{error}</Text>
            {failedResults && (
              <Button disabled={busy || loading} onClick={() => void retryResults()}>
                Retry loading results
              </Button>
            )}
            {!history.length && !loading && (
              <Button
                disabled={busy}
                onClick={() => {
                  setLoading(true);
                  setError(null);
                  setReload((value) => value + 1);
                }}
              >
                Retry loading scans
              </Button>
            )}
          </Box>
        )}
        {section === 'Duplicates' && viewedJob && (
          <DuplicateStudio
            key={`${viewedJob.id}:${groups?.groups[0]?.id ?? 'empty'}`}
            job={viewedJob}
            groups={groups}
            findings={files}
            status={status}
            next={() => void nextGroups()}
          />
        )}
        {section === 'Storage' ? (
          <>
            {storageDetailsOpen ? (
              <InspectionDialog
                title="Storage measurement"
                onClose={() => {
                  setStorageDetailsOpen(false);
                  storageDetailsOpener.current?.focus();
                }}
              >
                {error && (
                  <Text
                    role="alert"
                    className="error"
                    css={{
                      border: '1px solid {colors.border.control}',
                      borderRadius: 'var(--control-radius)',
                      background: '{colors.danger.bg}',
                      color: '{colors.danger.fg}',
                      padding: '16px 20px',
                      margin: '24px 0 0',
                      fontSize: '13px',
                      lineHeight: '1.6',
                      '& p': { margin: '0' },
                      '& button': {
                        marginTop: '12px',
                        border: '1px solid currentColor',
                        background: 'transparent',
                        color: 'inherit',
                        borderRadius: '4px',
                        padding: '7px 12px',
                      },
                    }}
                  >
                    {error}
                  </Text>
                )}
                {inspectionDetails}
              </InspectionDialog>
            ) : (
              !viewedJob && inspectionDetails
            )}
            {viewedJob && report?.storage && files && (
              <StorageExplorer
                key={viewedJob.id}
                jobId={viewedJob.id}
                summary={report.storage}
                volumes={report.volumes}
                roots={status.roots}
                firstPage={files}
              />
            )}
            {viewedJob && !report?.storage && !error && (
              <Text role="status">
                No storage observations available. Open Measurement details to review or rerun.
              </Text>
            )}
          </>
        ) : (
          inspectionDetails
        )}
        {files && section === 'Integrity' && (
          <chakra.section aria-label="Inspected files">
            <IntegrityFindings key={viewedJob?.id} rows={files.rows} />
            {files.next && <Button onClick={() => void nextFiles()}>Next file page</Button>}
          </chakra.section>
        )}
      </Box>
    </Box>
  );
}

function PropertyControls({
  rules,
  onChange,
  disabled,
}: {
  rules: PropertyRules;
  onChange: (rules: PropertyRules) => void;
  disabled: boolean;
}) {
  return (
    <chakra.fieldset>
      <chakra.legend>Advanced candidate rules</chakra.legend>
      <InspectionChoiceCards
        label="Combination"
        value={rules.logic}
        choices={[
          {
            value: 'all',
            label: 'All selected criteria',
            description: 'Every selected property must match (AND).',
          },
          {
            value: 'any',
            label: 'Any selected criterion',
            description: 'One selected property may match (OR). Candidates only.',
          },
        ]}
        disabled={disabled}
        onChange={(logic) => onChange({ ...rules, logic })}
      />
      <Box
        className="inspection-rule-options"
        css={{ display: 'flex', flexWrap: 'wrap', gap: '8px 20px', alignItems: 'center' }}
      >
        {(
          ['filename', 'extension', 'size', 'modified', 'created', 'readonly', 'content'] as const
        ).map((key) => (
          <label key={key}>
            <input
              type="checkbox"
              checked={rules[key]}
              onChange={(event) => onChange({ ...rules, [key]: event.target.checked })}
            />
            {
              {
                filename: 'Base filename',
                extension: 'Extension',
                size: 'Logical size',
                modified: 'Filesystem modified time',
                created: 'Filesystem creation/birth time',
                readonly: 'Read-only attribute',
                content: 'Complete byte equality',
              }[key]
            }
          </label>
        ))}
      </Box>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Date tolerance (milliseconds)</span>
          </Field.Label>
          <Input
            type="number"
            min="0"
            max="86400000"
            step="1"
            value={rules.toleranceMillis}
            onChange={(event) =>
              onChange({ ...rules, toleranceMillis: Number(event.target.value) })
            }
          />
        </label>
      </Field.Root>
      <Text>Missing dates or attributes are unknown. Dates do not use EXIF or Unix ctime.</Text>
      <label>
        <input
          type="checkbox"
          checked={rules.name.lowercase}
          onChange={(event) =>
            onChange({ ...rules, name: { ...rules.name, lowercase: event.target.checked } })
          }
        />
        Compare lowercase keys
      </label>
      <label>
        <input
          type="checkbox"
          checked={rules.name.unicodeNfc}
          onChange={(event) =>
            onChange({ ...rules, name: { ...rules.name, unicodeNfc: event.target.checked } })
          }
        />
        Unicode NFC comparison keys
      </label>
      <label>
        <input
          type="checkbox"
          checked={rules.name.ignoreWhitespace}
          onChange={(event) =>
            onChange({ ...rules, name: { ...rules.name, ignoreWhitespace: event.target.checked } })
          }
        />
        Ignore whitespace in comparison keys
      </label>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Literal characters to ignore</span>
          </Field.Label>
          <Input
            value={rules.name.ignoredCharacters}
            maxLength={128}
            onChange={(event) =>
              onChange({ ...rules, name: { ...rules.name, ignoredCharacters: event.target.value } })
            }
          />
        </label>
      </Field.Root>
      <Text>
        Comparison keys are shown in the table. Original names remain unchanged; empty keys are
        unknown.
      </Text>
    </chakra.fieldset>
  );
}
