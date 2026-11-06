import { PageHeader } from '@/components/PageHeader';
import { Field } from '@chakra-ui/react';
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
import '../../inspection/inspection.css';

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
  const epoch = useRef(0);
  const running = active(job);
  const jobId = job?.id ?? null;
  const loadResults = useCallback(async (record: InspectionJob, token: number) => {
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
  }, [section, loadResults]);
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
      <details className="inspection-setup" open={history.length === 0 || setupOpen}>
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
        <fieldset
          disabled={busy || running}
          className={`inspection-controls ${section === 'Duplicates' ? 'duplicate-controls' : ''}`}
        >
          <legend className={section === 'Duplicates' ? 'screen-reader-status' : undefined}>
            {section === 'Duplicates'
              ? 'Find duplicate clusters'
              : section === 'Integrity'
                ? 'Check file health'
                : 'Measure storage usage'}
          </legend>
          {section === 'Duplicates' && (
            <DuplicateMatchOptions
              options={options}
              onChange={setOptions}
              disabled={busy || running}
            />
          )}
          <div className="inspection-roots">
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
          </div>
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
          <p>
            Your hidden-file, hidden-folder, filename-pattern and size exclusions still apply.
            Gallery file-type and date filters do not limit this inspection. Links, special files
            and cloud placeholders are reported, never followed or downloaded.
          </p>
          {section === 'Duplicates' && (
            <>
              <p>
                Exact matches are confirmed by comparing every byte. Matching names, sizes or other
                properties only identify candidates; they never authorize quarantine.
              </p>
              <div>
                {options.mode === 'properties' && (
                  <PropertyControls
                    rules={options.rules}
                    disabled={busy || running}
                    onChange={(rules) => setOptions({ ...options, rules })}
                  />
                )}
              </div>
            </>
          )}
          {unavailable && (
            <p role="status">
              Reconnect selected unavailable folders before inspecting. Their retained inventory is
              not an empty folder.
            </p>
          )}
          {invalidRules && <p role="status">Choose at least one property criterion.</p>}
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
              <p>
                Checksums read complete file bytes. A mismatch means changed since baseline, not
                proven damage. Previous baselines remain in history; these checks do not establish
                backup or original-file recovery.
              </p>
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
        </fieldset>
      </details>
      {job && (
        <section className="inspection-job" aria-label="Inspection job">
          <h2>{job.state.replaceAll('_', ' ')}</h2>
          <p role="status">{job.message}</p>
          <p>
            {job.filesObserved.toLocaleString()} observed · {job.filesChecked.toLocaleString()}{' '}
            examined · {displayBytes(job.bytesRead)} read · {job.issues.toLocaleString()} issues
          </p>
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
          {job.limited && (
            <p>Inspection reached a bound. These results do not cover the whole selected scope.</p>
          )}
          <p>
            Retained scope: {job.input.scope.replaceAll('_', ' ')} ·{' '}
            {job.input.rootIds
              .map(
                (id) =>
                  status.roots.find((root) => root.id === id)?.label ??
                  `Previously registered folder ${id.slice(0, 8)}`,
              )
              .join(', ')}
            . Re-run to check current files; retained results grant no mutation approval.
          </p>
        </section>
      )}
      {report && (
        <>
          {viewedJob && <InspectionIssues key={viewedJob.id} jobId={viewedJob.id} />}
          {section === 'Duplicates' && (
            <details>
              <summary>Format applicability</summary>
              <p>{report.id3Applicability}</p>
            </details>
          )}
          <p className="inspection-coverage">
            Inventory:{' '}
            {report.coverage.enumerationComplete
              ? 'completed for this eligible scope'
              : 'incomplete'}{' '}
            · {report.coverage.excludedEntries} excluded entries/scopes ·{' '}
            {report.coverage.inaccessibleEntries} inaccessible/unsupported entries ·{' '}
            {report.coverage.incompleteDirectories} incomplete directories.
          </p>
        </>
      )}
    </>
  );
  const versionLabel = viewedJob && (
    <p>
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
    </p>
  );
  return (
    <div className={`inspection-page inspection-section-${section.toLowerCase()}`}>
      <PageHeader>
        <header className="toolbar">
          {navigationControl}
          <Box minW="0">
            <h1>{section}</h1>
          </Box>
          <div className="header-actions">
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
          </div>
        </header>
      </PageHeader>
      <div className="inspection-content">
        {section === 'Storage' && (
          <>
            {notice && <p role="status">{notice}</p>}
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
        {loading && <p role="status">Loading saved scans…</p>}
        {section !== 'Storage' && (
          <>
            {notice && <p role="status">{notice}</p>}
            {versionLabel}
            <p>
              {section === 'Duplicates'
                ? 'Find identical files or compare file properties. Review the matches while your originals stay unchanged.'
                : 'Check integrity without rewriting or repairing originals.'}
            </p>
          </>
        )}
        {error && (section !== 'Storage' || !storageDetailsOpen) && (
          <p role="alert" className="error">
            {error}
          </p>
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
                  <p role="alert" className="error">
                    {error}
                  </p>
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
              <p role="status">
                No storage observations available. Open Measurement details to review or rerun.
              </p>
            )}
          </>
        ) : (
          inspectionDetails
        )}
        {files && section === 'Integrity' && (
          <section aria-label="Inspected files">
            <IntegrityFindings key={viewedJob?.id} rows={files.rows} />
            {files.next && <Button onClick={() => void nextFiles()}>Next file page</Button>}
          </section>
        )}
      </div>
    </div>
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
    <fieldset>
      <legend>Advanced candidate rules</legend>
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
      <div className="inspection-rule-options">
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
      </div>
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
      <p>Missing dates or attributes are unknown. Dates do not use EXIF or Unix ctime.</p>
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
      <p>
        Comparison keys are shown in the table. Original names remain unchanged; empty keys are
        unknown.
      </p>
    </fieldset>
  );
}
