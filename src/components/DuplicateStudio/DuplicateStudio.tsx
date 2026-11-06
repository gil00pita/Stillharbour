import { useRef, useState, type MouseEvent } from 'react';
import { Box, Field, useRecipe } from '@chakra-ui/react';
import type { DuplicatePage, FindingPage, InspectionJob, LibraryStatus } from '../../bindings';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { displayBytes } from '../../inspection/format';
import { evidenceLabels, initialCleanupPolicy } from './model';
import { DuplicateDetail } from './components/DuplicateDetail';
import { CleanupRules } from './components/CleanupRules';
import { ReviewDialog } from './components/ReviewDialog';
import { duplicateStudioRecipe } from './DuplicateStudio.recipe';

export function DuplicateStudio({
  job,
  groups,
  findings,
  status,
  next,
}: {
  job: InspectionJob | null;
  groups: DuplicatePage | null;
  findings: FindingPage | null;
  status: LibraryStatus;
  next: () => void;
}) {
  const styles = useRecipe({ recipe: duplicateStudioRecipe })();
  const rulesOpener = useRef<HTMLButtonElement | null>(null);
  const recentOpener = useRef<HTMLButtonElement | null>(null);
  function openRules(event: MouseEvent<HTMLButtonElement>) {
    rulesOpener.current = event.currentTarget;
    setRules(true);
  }
  const [policy, setPolicy] = useState(initialCleanupPolicy);
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState('All groups');
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [selected, setSelected] = useState(new Set<string>());
  const [rules, setRules] = useState(false);
  const [recent, setRecent] = useState(false);
  const inspecting = Boolean(job && ['queued', 'running', 'cancel_requested'].includes(job.state));
  const rows = groups?.groups ?? [];
  const matches = rows.filter((group) => {
    const names =
      findings?.rows
        .filter((row) => group.memberIds.includes(row.id))
        .map((row) => `${row.name} ${row.displayPath}`)
        .join(' ') ?? '';
    return (
      (filter === 'All groups' ||
        (filter === 'Exact matches' && group.evidence === 'exact_bytes') ||
        (filter === 'Needs review' && group.evidence !== 'exact_bytes')) &&
      `${evidenceLabels[group.evidence]} ${group.id} ${names}`
        .toLowerCase()
        .includes(search.toLowerCase())
    );
  });
  const current = matches.find((group) => group.id === selectedId) ?? matches[0];
  const exact = rows.filter((group) => group.evidence === 'exact_bytes');
  const candidates = rows
    .filter((group) => group.evidence !== 'hard_link_aliases')
    .reduce((total, group) => total + Math.max(0, group.memberCount - 1), 0);
  return (
    <Box as="section" css={styles} className="duplicate-studio" aria-label="Duplicate groups">
      <div className="duplicate-summary">
        <div>
          <h2>Make room for the photos that matter.</h2>
          <p>Compare the evidence, choose what stays, and keep uncertain versions together.</p>
        </div>
        <div className="duplicate-summary-actions">
          <Button
            onClick={(event) => {
              recentOpener.current = event.currentTarget;
              setRecent(true);
            }}
          >
            Recently Deleted
          </Button>
          <Button onClick={openRules}>Cleanup rules</Button>
        </div>
      </div>
      <dl className="duplicate-totals">
        <div>
          <dt>Groups on this page</dt>
          <dd>{rows.length}</dd>
        </div>
        <div>
          <dt>Potential duplicates</dt>
          <dd>{candidates}</dd>
        </div>
        <div>
          <dt>Exact-match groups</dt>
          <dd>{exact.length}</dd>
        </div>
        <div>
          <dt>Recoverable storage</dt>
          <dd>Not verified</dd>
        </div>
      </dl>
      <p className="duplicate-safety-note">
        Read-only review · Cleanup needs fresh byte verification, protected companions and verified
        independent recovery. Recently Deleted retains bytes and frees {displayBytes('0')}.
      </p>
      <div className="duplicate-workspace">
        <div className="duplicate-queue">
          <div className="duplicate-queue-head">
            <h3>Duplicate clusters</h3>
            <Field.Root>
              <Field.Label className="screen-reader-status">Search duplicate groups</Field.Label>
              <Input
                type="search"
                placeholder="Search observed files or evidence…"
                maxLength={256}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
            </Field.Root>
            <div className="duplicate-filters" role="group" aria-label="Duplicate filters">
              {['All groups', 'Exact matches', 'Needs review', 'Can automate'].map((item) => (
                <Button key={item} aria-pressed={filter === item} onClick={() => setFilter(item)}>
                  {item}
                </Button>
              ))}
            </div>
          </div>
          {rows.length > 0 && (
            <div className="duplicate-select-all">
              <label>
                <input
                  type="checkbox"
                  checked={matches.length > 0 && matches.every((group) => selected.has(group.id))}
                  disabled={!matches.length}
                  onChange={(event) =>
                    setSelected(
                      event.target.checked ? new Set(matches.map((group) => group.id)) : new Set(),
                    )
                  }
                />{' '}
                Select shown groups
              </label>
              <span>{selected.size} selected</span>
            </div>
          )}
          <div className="duplicate-clusters">
            {matches.map((group, index) => {
              const observed = findings?.rows.find((row) => group.memberIds.includes(row.id));
              return (
                <article
                  key={group.id}
                  className={`duplicate-cluster ${group.id === current?.id ? 'is-current' : ''}`}
                >
                  <label className="duplicate-cluster-select">
                    <input
                      type="checkbox"
                      aria-label={`Select group ${index + 1}`}
                      checked={selected.has(group.id)}
                      onChange={(event) =>
                        setSelected((previous) => {
                          const value = new Set(previous);
                          if (event.target.checked) value.add(group.id);
                          else value.delete(group.id);
                          return value;
                        })
                      }
                    />
                  </label>
                  <Button
                    aria-pressed={group.id === current?.id}
                    onClick={() => setSelectedId(group.id)}
                  >
                    <span className="duplicate-cluster-number" aria-hidden="true">
                      {String(index + 1).padStart(2, '0')}
                    </span>
                    <span>
                      <strong>
                        <bdi>{observed?.name ?? evidenceLabels[group.evidence]}</bdi>
                      </strong>
                      <span>
                        {group.memberCount} files ·{' '}
                        {group.evidence === 'hard_link_aliases'
                          ? 'Shared object'
                          : `${Math.max(0, group.memberCount - 1)} potential duplicate${group.memberCount === 2 ? '' : 's'}`}
                      </span>
                      <small>{evidenceLabels[group.evidence]}</small>
                      <small>Visual confidence: not measured</small>
                    </span>
                  </Button>
                </article>
              );
            })}
            {!matches.length && (
              <div className="duplicate-empty">
                <h3>
                  {filter === 'Can automate'
                    ? 'Automatic cleanup is unavailable'
                    : rows.length
                      ? 'No groups match'
                      : inspecting
                        ? 'Finding duplicate clusters…'
                        : job
                          ? 'No completed duplicate groups'
                          : 'Start with a duplicate inspection'}
                </h3>
                <p>
                  {filter === 'Can automate'
                    ? 'Even exact matches need a fresh surviving keeper, companion checks and independent recovery. No groups can be automated in this build.'
                    : rows.length
                      ? 'Try another search or filter. Search covers the observed file names loaded on this page.'
                      : inspecting
                        ? 'Comparing the selected folders. Progress and cancellation are available in Jobs; originals stay unchanged.'
                        : job
                          ? 'No matching groups are retained in this inspection. Incomplete and unavailable scope remains unknown.'
                          : 'Choose folders and a matching method above. Your originals stay in place.'}
                </p>
              </div>
            )}
          </div>
          {groups?.next && <Button onClick={next}>Next group page</Button>}
          {selected.size > 0 && (
            <Button onClick={openRules}>Review rules for {selected.size} groups</Button>
          )}
        </div>
        <div className="duplicate-inspector">
          {current && job ? (
            <DuplicateDetail
              key={`${job.id}:${current.id}`}
              jobId={job.id}
              group={current}
              status={status}
              rule={policy.rule}
              onRule={(rule) => setPolicy((previous) => ({ ...previous, rule }))}
            />
          ) : (
            <div className="duplicate-empty">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path d="M4 4h11v13H4ZM9 8h11v13H9" stroke="currentColor" strokeWidth="1" />
              </svg>
              <h3>Every version has a story</h3>
              <p>Choose a cluster to inspect its files, matching evidence and keeper choices.</p>
            </div>
          )}
        </div>
      </div>
      {rules && (
        <CleanupRules
          policy={policy}
          onPolicy={setPolicy}
          groups={rows.filter((group) => !selected.size || selected.has(group.id))}
          onClose={() => {
            setRules(false);
            rulesOpener.current?.focus();
          }}
        />
      )}
      {recent && (
        <RecentlyDeleted
          onClose={() => {
            setRecent(false);
            recentOpener.current?.focus();
          }}
        />
      )}
    </Box>
  );
}

function RecentlyDeleted({ onClose }: { onClose: () => void }) {
  return (
    <ReviewDialog title="Recently Deleted" onClose={onClose}>
      <h3>No files have been moved</h3>
      <p>
        Duplicate removal and Undo are unavailable until the native cleanup and recovery gates pass.
        A retained keeper review does not move files here.
      </p>
      <p>
        When available, removal will preserve files and their original paths without automatic
        expiry. Restoring will require a new collision-safe plan. Quarantine does not reclaim
        storage.
      </p>
      <Button disabled>Undo removal unavailable</Button>
    </ReviewDialog>
  );
}
