import { Button, Input, Select } from '../components/ui/controls';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import type {
  AssetSummary,
  HealthMode,
  DuplicateOptions,
  DuplicatePage,
  FindingPage,
  InspectionJob,
  InspectionReport,
  LibraryStatus,
  PropertyRules,
} from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { ComparisonViewer } from '../library/ComparisonViewer';
import { StorageReview } from './StorageReview';
import { KeeperControls } from './KeeperControls';
import { InspectionIssues } from './InspectionIssues';
import { displayBytes } from './format';
import './inspection.css';

export type InspectionSection = 'Duplicates' | 'Integrity' | 'Storage';
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
      libraryData(
        { kind: 'inspection_groups', job_id: record.id, after: null },
        'inspection_groups',
      ),
    ]);
    if (token !== epoch.current) return;
    setReport(summary);
    setFiles(rows);
    setGroups(matches);
  }, []);
  useEffect(() => {
    const token = epoch.current;
    let disposed = false;
    void libraryData({ kind: 'inspection_jobs' }, 'inspection_jobs')
      .then(async (jobs) => {
        if (disposed || token !== epoch.current) return;
        const matching = jobs.filter((record) => taskMatches(record, section));
        setHistory(matching);
        if (matching[0]) {
          setJob(matching[0]);
          if (!active(matching[0])) await loadResults(matching[0], token);
        }
      })
      .catch((failure) => {
        if (!disposed && token === epoch.current) setError(errorMessage(failure));
      });
    return () => {
      disposed = true;
    };
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
          await loadResults(next, token);
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
    setReport(null);
    setFiles(null);
    setGroups(null);
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
        setHistory((previous) => [next, ...previous].slice(0, 64));
      }
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
    } finally {
      if (token === epoch.current) setBusy(false);
    }
  }
  async function selectJob(record: InspectionJob) {
    if (record.id === job?.id) return;
    const token = ++epoch.current;
    setJob(record);
    setReport(null);
    setFiles(null);
    setGroups(null);
    setError(null);
    try {
      if (!active(record)) await loadResults(record, token);
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
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
    if (!job || !files?.next) return;
    const token = epoch.current;
    try {
      const page = await libraryData(
        { kind: 'inspection_findings', job_id: job.id, after: files.next },
        'inspection_findings',
      );
      if (token === epoch.current) setFiles(page);
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
    }
  }
  async function nextGroups() {
    if (!job || !groups?.next) return;
    const token = epoch.current;
    try {
      const page = await libraryData(
        { kind: 'inspection_groups', job_id: job.id, after: groups.next },
        'inspection_groups',
      );
      if (token === epoch.current) setGroups(page);
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
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
  return (
    <div className="inspection-page">
      <header className="toolbar">
        {navigationControl}
        <h1>{section}</h1>
        <div className="header-actions">{jobsControl}</div>
      </header>
      <div className="inspection-content">
        <p>
          {section === 'Duplicates'
            ? 'Find identical files or compare file properties. Review the matches while your originals stay unchanged.'
            : section === 'Storage'
              ? 'See how much space files and folders use in your selected libraries. File size alone does not tell you how much disk space can be freed.'
              : 'Check integrity without rewriting or repairing originals.'}
        </p>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <fieldset disabled={busy || running} className="inspection-controls">
          <legend>New inspection</legend>
          <div className="inspection-roots">
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
          <label>
            File scope
            <Select
              value={scope}
              onChange={(event) => setScope(event.target.value as typeof scope)}
            >
              <option value="indexed_media_companions">
                Indexed photos, videos and matching companion files
              </option>
              <option value="all_eligible_files">All files within the exclusions below</option>
            </Select>
          </label>
          <p>
            Your hidden-file, hidden-folder, filename-pattern and size exclusions still apply.
            Gallery file-type and date filters do not limit this inspection. Links, special files
            and cloud placeholders are reported, never followed or downloaded.
          </p>
          {section === 'Duplicates' && (
            <>
              <label>
                Match mode
                <Select
                  value={options.mode}
                  onChange={(event) =>
                    setOptions({ ...options, mode: event.target.value as DuplicateOptions['mode'] })
                  }
                >
                  <option value="exact_content">Identical file contents</option>
                  <option value="properties">Matching file properties · candidates only</option>
                  <option value="normalized_pixels">Static PNG8 pixels · manual review</option>
                </Select>
              </label>
              <label>
                Content algorithm
                <Select
                  value={options.algorithm}
                  onChange={(event) =>
                    setOptions({
                      ...options,
                      algorithm: event.target.value as DuplicateOptions['algorithm'],
                    })
                  }
                >
                  <option value="blake3">BLAKE3</option>
                  <option value="sha256">SHA-256</option>
                  <option value="sha1">SHA-1 · compatibility only</option>
                  <option value="md5">MD5 · compatibility only</option>
                  <option value="direct_bytes">Direct bytes · no full-file hash</option>
                </Select>
              </label>
              <p>
                Exact matches are confirmed by comparing every byte. Matching names, sizes or other
                properties only identify candidates; they never authorize quarantine.
              </p>
              <label>
                <input
                  type="checkbox"
                  checked={options.excludeHardlinkedObjects}
                  onChange={(event) =>
                    setOptions({ ...options, excludeHardlinkedObjects: event.target.checked })
                  }
                />
                Exclude hard-linked objects from this search
              </label>
              {options.mode === 'properties' && (
                <PropertyControls
                  rules={options.rules}
                  onChange={(rules) => setOptions({ ...options, rules })}
                />
              )}
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
              <label>
                Integrity mode
                <Select
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
                </Select>
              </label>
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
        {history.length > 0 && (
          <label>
            Retained inspection
            <Select
              disabled={busy}
              value={job?.id ?? ''}
              onChange={(event) => {
                const record = history.find((record) => record.id === event.target.value);
                if (record) void selectJob(record);
              }}
            >
              {history.map((record) => (
                <option key={record.id} value={record.id}>
                  {record.startedAt} · {record.state}
                </option>
              ))}
            </Select>
          </label>
        )}
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
              <p>
                Inspection reached a bound. These results do not cover the whole selected scope.
              </p>
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
            {job && <InspectionIssues key={job.id} jobId={job.id} />}
            <p>{report.id3Applicability}</p>
            <p className="inspection-coverage">
              Inventory:{' '}
              {report.coverage.enumerationComplete
                ? 'completed for this eligible scope'
                : 'incomplete'}{' '}
              · {report.coverage.excludedEntries} excluded entries/scopes ·{' '}
              {report.coverage.inaccessibleEntries} inaccessible/unsupported entries ·{' '}
              {report.coverage.incompleteDirectories} incomplete directories.
            </p>
            <div className="inspection-table-wrap">
              <table className="inspection-volumes">
                <caption>
                  Observed actual volumes · capacity is separate from selected-scope totals
                </caption>
                <thead>
                  <tr>
                    <th>Registered roots on volume</th>
                    <th>Total bytes</th>
                    <th>Available bytes</th>
                    <th>Observed</th>
                  </tr>
                </thead>
                <tbody>
                  {report.volumes.map((volume) => (
                    <tr key={volume.id}>
                      <th scope="row">
                        {volume.rootIds
                          .map(
                            (id) =>
                              status.roots.find((root) => root.id === id)?.label ?? id.slice(0, 8),
                          )
                          .join(', ')}
                      </th>
                      <td>{displayBytes(volume.totalBytes)}</td>
                      <td>{displayBytes(volume.availableBytes)}</td>
                      <td>
                        {volume.observedAt}
                        <br />
                        {volume.coverage}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {report.storage && job && (
              <StorageReview key={job.id} jobId={job.id} summary={report.storage} />
            )}
          </>
        )}
        {job && groups && groups.groups.length > 0 && (
          <section aria-label="Duplicate groups">
            <h2>Reviewed evidence groups</h2>
            <p>
              Exact bytes, property candidates and linked paths remain separate. Quarantine is
              unavailable until independent exact-version recovery and applicable native gates pass.
            </p>
            {groups.groups.map((group) => (
              <GroupReview
                key={`${job.id}:${group.id}`}
                jobId={job.id}
                group={group}
                status={status}
              />
            ))}
            {groups.next && <Button onClick={() => void nextGroups()}>Next group page</Button>}
          </section>
        )}
        {job && report && groups?.groups.length === 0 && job.input.task.kind === 'duplicates' && (
          <p>
            {job.state === 'completed'
              ? 'No matching groups were reported in this inspected scope.'
              : 'No completed groups are retained in this partial inspection; unavailable scope remains unknown.'}
          </p>
        )}
        {files && (
          <section aria-label="Inspected files">
            <h2>Files · largest logical size first</h2>
            <FindingTable rows={files.rows} />
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
}: {
  rules: PropertyRules;
  onChange: (rules: PropertyRules) => void;
}) {
  return (
    <fieldset>
      <legend>Advanced candidate rules</legend>
      <label>
        Combination
        <Select
          value={rules.logic}
          onChange={(event) =>
            onChange({ ...rules, logic: event.target.value as PropertyRules['logic'] })
          }
        >
          <option value="all">All selected criteria (AND)</option>
          <option value="any">Any selected criterion (OR) · candidates only</option>
        </Select>
      </label>
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
      <label>
        Date tolerance (milliseconds)
        <Input
          type="number"
          min="0"
          max="86400000"
          step="1"
          value={rules.toleranceMillis}
          onChange={(event) => onChange({ ...rules, toleranceMillis: Number(event.target.value) })}
        />
      </label>
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
      <label>
        Literal characters to ignore
        <Input
          value={rules.name.ignoredCharacters}
          maxLength={128}
          onChange={(event) =>
            onChange({ ...rules, name: { ...rules.name, ignoredCharacters: event.target.value } })
          }
        />
      </label>
      <p>
        Comparison keys are shown in the table. Original names remain unchanged; empty keys are
        unknown.
      </p>
    </fieldset>
  );
}

function FindingTable({ rows }: { rows: FindingPage['rows'] }) {
  return (
    <div className="inspection-table-wrap">
      <table>
        <caption>Retained file observations</caption>
        <thead>
          <tr>
            <th>File</th>
            <th>Logical bytes</th>
            <th>Allocation</th>
            <th>Inventory / linked object</th>
            <th>Content and coverage</th>
            <th>Checksum</th>
            <th>Backup</th>
            <th>Comparison key</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id}>
              <th scope="row">
                <span>{row.name}</span>
                <small>{row.displayPath}</small>
                {row.error && <small>{row.error.message}</small>}
              </th>
              <td>{displayBytes(row.bytes)}</td>
              <td>{displayBytes(row.reportedAllocatedBytes)}</td>
              <td>
                {row.inventory.replaceAll('_', ' ')}
                <br />
                {row.links} links · {row.objectId?.slice(0, 16) ?? 'Object unknown'}
              </td>
              <td>
                {row.content.replaceAll('_', ' ')}
                <br />
                {row.format ?? 'Format unknown'}
                {row.width != null && row.height != null ? ` · ${row.width} × ${row.height}` : ''}
                <br />
                {row.coverage}
                {row.pixelSignature && (
                  <details>
                    <summary>Declared pixel policy and candidate fingerprint</summary>
                    <p>{row.pixelSignature}</p>
                    <code>{row.pixelDigest}</code>
                    <p>
                      Fingerprint match is manual-review evidence and grants no cleanup authority.
                    </p>
                  </details>
                )}
                {row.error && <p>{row.error.message}</p>}
              </td>
              <td>{row.checksum.replaceAll('_', ' ')}</td>
              <td>Not assessed · M4 recovery unavailable</td>
              <td>{row.comparisonName ?? 'Unknown / not requested'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function GroupReview({
  jobId,
  group,
  status,
}: {
  jobId: string;
  group: DuplicatePage['groups'][number];
  status: LibraryStatus;
}) {
  const [members, setMembers] = useState<FindingPage | null>(null);
  const [keep, setKeep] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [comparison, setComparison] = useState<AssetSummary[] | null>(null);
  const generation = useRef(0);
  useEffect(
    () => () => {
      generation.current += 1;
    },
    [],
  );
  async function load(after: string | null = null) {
    const token = ++generation.current;
    try {
      const rows = await libraryData(
        { kind: 'inspection_group_members', job_id: jobId, group_id: group.id, after },
        'inspection_findings',
      );
      if (token === generation.current) {
        setMembers(rows);
        setError(null);
      }
    } catch (failure) {
      if (token === generation.current) setError(errorMessage(failure));
    }
  }
  async function compare() {
    const selected = members?.rows.filter((row) => row.isMedia && row.assetId).slice(0, 4) ?? [];
    const token = ++generation.current;
    try {
      const details = await Promise.all(
        selected.map((row) => libraryData({ kind: 'detail', asset_id: row.assetId! }, 'detail')),
      );
      if (token === generation.current) setComparison(details.map((detail) => detail.asset));
    } catch (failure) {
      if (token === generation.current) setError(errorMessage(failure));
    }
  }
  return (
    <article className="inspection-group">
      <h3>
        {group.evidence.replaceAll('_', ' ')} · {group.memberCount.toLocaleString()} paths
      </h3>
      <p>{group.explanation}</p>
      {error && <p role="alert">{error}</p>}
      <div className="inspection-actions">
        <Button onClick={() => void load()}>Review group members</Button>
        <Button
          disabled={(members?.rows.filter((row) => row.isMedia && row.assetId).length ?? 0) < 2}
          onClick={() => void compare()}
        >
          Compare photo previews
        </Button>
        <Button
          disabled
          title="Independent recovery and fresh complete bundle verification are required"
        >
          Quarantine unavailable
        </Button>
      </div>
      {members && (
        <>
          <p>
            Choose manual keepers below, then retain a backend review. Keep multiple when uncertain;
            no files are moved. Incomplete or unique companions require separate verification.
          </p>
          <div className="inspection-pair">
            {members.rows.slice(0, 4).map((row) => (
              <section key={row.id}>
                <h4>{row.name}</h4>
                <p>{row.displayPath}</p>
                <dl>
                  <dt>Logical bytes</dt>
                  <dd>{displayBytes(row.bytes)}</dd>
                  <dt>Filesystem modified</dt>
                  <dd>{row.filesystemModified ?? 'Unknown'}</dd>
                  <dt>Filesystem created/birth</dt>
                  <dd>{row.filesystemCreated ?? 'Unknown'}</dd>
                  <dt>Linked object</dt>
                  <dd>
                    {row.objectId?.slice(0, 16) ?? 'Unknown'} · {row.links} links
                  </dd>
                </dl>
                <label>
                  <input
                    type="checkbox"
                    checked={keep.has(row.id)}
                    onChange={(event) => {
                      const next = new Set(keep);
                      if (event.target.checked) next.add(row.id);
                      else next.delete(row.id);
                      setKeep(next);
                    }}
                  />
                  Keep in this review
                </label>
              </section>
            ))}
          </div>
          <KeeperControls
            jobId={jobId}
            groupId={group.id}
            rows={members.rows}
            keep={keep}
            onKeep={setKeep}
          />
          <FindingTable rows={members.rows} />
          {members.next && (
            <Button onClick={() => void load(members.next)}>Next member page</Button>
          )}
        </>
      )}
      {comparison && (
        <ComparisonViewer
          returnLabel="Back to duplicate review"
          assets={comparison}
          status={status}
          onClose={() => setComparison(null)}
          onError={(failure) => setError(errorMessage(failure))}
        />
      )}
    </article>
  );
}
