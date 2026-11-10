import { useEffect, useRef, useState } from 'react';
import { Box, Field, useRecipe, Heading, Table, Text, chakra } from '@chakra-ui/react';
import type {
  FindingPage,
  InspectionFinding,
  InspectionVolume,
  LibraryRoot,
  StorageSummary,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { displayBytes } from '../../inspection/format';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { StorageReview } from '@/components/StorageReview';
import { storageExplorerRecipe } from './StorageExplorer.recipe';
import { StorageOverview } from './components/StorageOverview';
import { StorageMap, type FileGroup } from './components/StorageMap';
import {
  ageColors,
  ageOf,
  cameraOf,
  categories,
  categoryOf,
  colorOf,
  emptyFilters,
  matches,
  pathParts,
  percentage,
  totalBytes,
  type CameraRecords,
  type Filters,
} from './model';

const maxFiles = 2048;
export function StorageExplorer({
  jobId,
  summary,
  volumes,
  roots,
  firstPage,
}: {
  jobId: string;
  summary: StorageSummary;
  volumes: InspectionVolume[];
  roots: LibraryRoot[];
  firstPage: FindingPage;
}) {
  const styles = useRecipe({ recipe: storageExplorerRecipe })();
  const [page, setPage] = useState(firstPage);
  const [filters, setFilters] = useState<Filters>(emptyFilters);
  const [mode, setMode] = useState('File type');
  const [grouping, setGrouping] = useState('Folder');
  const [view, setView] = useState('Map');
  const [expanded, setExpanded] = useState(false);
  const [folderTotalsOpen, setFolderTotalsOpen] = useState(false);
  const [trail, setTrail] = useState<string[]>([]);
  const [focusedFiles, setFocusedFiles] = useState<Set<string> | null>(null);
  const [selection, setSelection] = useState<InspectionFinding | FileGroup | null>(null);
  const [tray, setTray] = useState<InspectionFinding[]>([]);
  const [simulate, setSimulate] = useState(false);
  const [cameras, setCameras] = useState<CameraRecords>({});
  const [cameraAttempt, setCameraAttempt] = useState(0);
  const cameraCache = useRef<CameraRecords>({});
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const moreInFlight = useRef(false);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);
  useEffect(() => {
    let active = true;
    const queue = page.rows.filter(
      (file) => file.isMedia && file.assetId && !cameraCache.current[file.id],
    );
    async function worker() {
      while (active) {
        const file = queue.shift();
        if (!file?.assetId) return;
        let record;
        try {
          const detail = await libraryData({ kind: 'detail', asset_id: file.assetId }, 'detail');
          record = { label: detail.asset.camera || null };
        } catch (failure) {
          record = { label: null, error: errorMessage(failure) };
        }
        if (!active) return;
        cameraCache.current[file.id] = record;
        setCameras((previous) => ({ ...previous, [file.id]: record }));
      }
    }
    void Promise.all([worker(), worker()]);
    return () => {
      active = false;
    };
  }, [page.rows, cameraAttempt]);
  async function more() {
    if (!page.next || moreInFlight.current || page.rows.length >= maxFiles) return;
    moreInFlight.current = true;
    setLoading(true);
    try {
      const next = await libraryData(
        { kind: 'inspection_findings', job_id: jobId, after: page.next },
        'inspection_findings',
      );
      if (mounted.current) {
        setPage((previous) => ({ ...next, rows: [...previous.rows, ...next.rows] }));
        setError(null);
      }
    } catch (failure) {
      if (mounted.current) setError(errorMessage(failure));
    } finally {
      moreInFlight.current = false;
      if (mounted.current) setLoading(false);
    }
  }
  function folderPath(file: InspectionFinding): string[] {
    const root = roots.find((root) => root.id === file.rootId);
    const prefix = root?.displayPath.replaceAll('\\', '/').split('/').filter(Boolean) ?? [];
    const parts = pathParts(file);
    const relative = prefix.every((part, index) => parts[index] === part)
      ? parts.slice(prefix.length)
      : parts;
    return [file.rootId, ...relative];
  }
  const scoped = page.rows.filter((file) =>
    trail.every((part, index) => folderPath(file)[index] === part),
  );
  const filtered = scoped.filter(
    (file) =>
      (!focusedFiles || focusedFiles.has(file.id)) &&
      matches(file, filters, cameras) &&
      (!simulate || !tray.some((item) => item.id === file.id)),
  );
  const bytes = totalBytes(filtered);
  const groups = new Map<string, FileGroup>();
  for (const file of filtered) {
    const path = folderPath(file);
    const next = path[trail.length];
    const key =
      grouping === 'File type'
        ? categoryOf(file)
        : grouping === 'Camera'
          ? cameraOf(file, cameras)
          : next
            ? JSON.stringify(path.slice(0, trail.length + 1))
            : 'direct-files';
    const name =
      grouping !== 'Folder'
        ? key
        : trail.length === 0
          ? (roots.find((root) => root.id === file.rootId)?.label ?? 'Previously registered root')
          : (next ?? 'Files in this folder');
    const group = groups.get(key) ?? {
      id: key,
      name,
      files: [],
      path: grouping === 'Folder' && next ? path.slice(0, trail.length + 1) : null,
    };
    group.files.push(file);
    groups.set(key, group);
  }
  // Folder diversity can exceed geometry's 512-node bound. Keep the remainder openable.
  const groupList = [...groups.values()];
  const mapGroups =
    groupList.length <= 128
      ? groupList
      : [
          ...groupList.slice(0, 127),
          {
            id: 'remaining-groups',
            name: 'More folders · open list',
            path: null,
            files: groupList.slice(127).flatMap((group) => group.files),
          },
        ];
  const knownCameras = [...new Set(page.rows.map((file) => cameraOf(file, cameras)))]
    .filter((camera) => camera !== 'Unknown camera' && camera !== 'Not camera media')
    .sort();
  const cameraFailures = Object.values(cameras).filter((record) => record.error).length;
  const selectedFile = selection && 'bytes' in selection ? selection : null;
  const selectedGroup = selection && 'files' in selection ? selection : null;
  const protectedFile =
    selectedFile &&
    (selectedFile.readonly !== false ||
      roots.find((root) => root.id === selectedFile.rootId)?.isBase);
  function update(key: keyof Filters, value: string) {
    setFocusedFiles(null);
    setFilters((previous) => ({ ...previous, [key]: value }));
    setSelection(null);
  }
  function reset() {
    setFocusedFiles(null);
    setFilters(emptyFilters);
    setTrail([]);
    setSimulate(false);
    setSelection(null);
  }
  function openGroup(group: FileGroup) {
    setFocusedFiles(null);
    if (group.path) {
      setTrail(group.path);
      setGrouping('Folder');
    } else if (grouping === 'File type') update('category', group.name);
    else if (grouping === 'Camera') update('camera', group.name);
    else setFocusedFiles(new Set(group.files.map((file) => file.id)));
    setSelection(null);
    if (!group.path) setView('List');
  }
  const legend =
    mode === 'File type'
      ? categories
      : mode === 'Age'
        ? Object.keys(ageColors)
        : ['Unknown camera', 'Not camera media', ...knownCameras];
  const sorted = [...filtered].sort((a, b) =>
    BigInt(a.bytes) === BigInt(b.bytes)
      ? a.id.localeCompare(b.id)
      : BigInt(a.bytes) > BigInt(b.bytes)
        ? -1
        : 1,
  );
  return (
    <Box
      css={[
        styles,
        {
          '& .inspection-table-wrap :is(th, td):not(:first-child)': {
            textAlign: 'right',
            fontVariantNumeric: 'tabular-nums',
          },
        },
      ]}
      className={`inspection-storage storage-explorer ${expanded ? 'storage-expanded' : ''}`}
    >
      <chakra.section className="storage-filters" aria-label="Storage filters">
        <Field.Root>
          <Field.Label>Search</Field.Label>
          <Input
            placeholder="Filename or folder path"
            value={filters.search}
            onChange={(event) => update('search', event.target.value)}
          />
        </Field.Root>
        <Field.Root>
          <Field.Label>File type</Field.Label>
          <NativeSelect
            value={filters.category}
            onChange={(event) => update('category', event.target.value)}
          >
            <option value="">All file types</option>
            {categories.map((category) => (
              <option key={category}>{category}</option>
            ))}
          </NativeSelect>
        </Field.Root>
        <Field.Root>
          <Field.Label>Camera</Field.Label>
          <NativeSelect
            value={filters.camera}
            onChange={(event) => update('camera', event.target.value)}
          >
            <option value="">All cameras</option>
            {['Known camera', 'Unknown camera', 'Not camera media', ...knownCameras].map(
              (camera) => {
                const entries = page.rows.filter((file) =>
                  matches(file, { ...emptyFilters, camera }, cameras),
                );
                return (
                  <option key={camera} value={camera}>
                    {camera} · {entries.length} files · {displayBytes(totalBytes(entries))}
                  </option>
                );
              },
            )}
          </NativeSelect>
        </Field.Root>
        <Field.Root>
          <Field.Label>Age · last modified</Field.Label>
          <NativeSelect value={filters.age} onChange={(event) => update('age', event.target.value)}>
            <option value="">Any modified date</option>
            {Object.keys(ageColors).map((age) => (
              <option key={age}>{age}</option>
            ))}
          </NativeSelect>
        </Field.Root>
        <Field.Root>
          <Field.Label>Minimum size</Field.Label>
          <NativeSelect
            value={filters.minimum}
            onChange={(event) => update('minimum', event.target.value)}
          >
            {[
              ['', 'Any size'],
              ['1000000', '1 MB'],
              ['100000000', '100 MB'],
              ['1000000000', '1 GB'],
              ['5000000000', '5 GB'],
              ['10000000000', '10 GB'],
            ].map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </NativeSelect>
        </Field.Root>
        <Button onClick={reset}>Reset</Button>
      </chakra.section>
      <Box className="storage-chips" aria-label="Active storage filters">
        {Object.entries(filters)
          .filter(([, value]) => value)
          .map(([key, value]) => (
            <Button
              key={key}
              aria-label={`Remove ${key} filter`}
              onClick={() => update(key as keyof Filters, '')}
            >
              {key === 'minimum' ? `At least ${displayBytes(value)}` : value} ×
            </Button>
          ))}
      </Box>
      <StorageOverview
        summary={summary}
        volumes={volumes}
        roots={roots}
        files={page.rows}
        complete={!page.next}
        selected={filters.category}
        onSelect={(value) => update('category', value)}
      />
      <Text className="storage-note">
        {page.rows.length.toLocaleString()} loaded of {summary.total.files.toLocaleString()}{' '}
        retained observations
        {page.next ? ' · Filters and file-type totals cover loaded observations only.' : '.'} Camera
        uses current recorded catalog metadata; missing, unread and unavailable metadata remains
        unknown. Age uses retained last-modified dates.
      </Text>
      {cameraFailures > 0 && (
        <Text role="status">
          Camera metadata unavailable for {cameraFailures} files.{' '}
          <Button
            onClick={() => {
              cameraCache.current = Object.fromEntries(
                Object.entries(cameraCache.current).filter(([, record]) => !record.error),
              );
              setCameraAttempt((value) => value + 1);
            }}
          >
            Retry camera metadata
          </Button>
        </Text>
      )}
      {error && <Text role="alert">{error}</Text>}
      {page.next && (
        <Button disabled={loading || page.rows.length >= maxFiles} onClick={() => void more()}>
          {loading
            ? 'Loading observations…'
            : page.rows.length >= maxFiles
              ? 'Observation display limit reached (2,048)'
              : 'Load more file observations'}
        </Button>
      )}
      <Box className="storage-toolbar">
        <strong>
          {filtered.length.toLocaleString()} files · {displayBytes(bytes)}
          {simulate ? ' · Simulation' : ''}
        </strong>
        <Field.Root>
          <Field.Label>Colour mode</Field.Label>
          <NativeSelect value={mode} onChange={(event) => setMode(event.target.value)}>
            {['File type', 'Age', 'Camera'].map((value) => (
              <option key={value}>{value}</option>
            ))}
          </NativeSelect>
        </Field.Root>
        <Field.Root>
          <Field.Label>Group by</Field.Label>
          <NativeSelect
            value={grouping}
            onChange={(event) => {
              setGrouping(event.target.value);
              setFocusedFiles(null);
              setSelection(null);
            }}
          >
            {['Folder', 'File type', 'Camera'].map((value) => (
              <option key={value}>{value}</option>
            ))}
          </NativeSelect>
        </Field.Root>
        <Box>
          {['Map', 'List'].map((value) => (
            <Button key={value} aria-pressed={view === value} onClick={() => setView(value)}>
              {value}
            </Button>
          ))}{' '}
          <Button aria-pressed={expanded} onClick={() => setExpanded((value) => !value)}>
            {expanded ? 'Reduce map' : 'Expand map'}
          </Button>
        </Box>
      </Box>
      <chakra.nav className="storage-breadcrumbs" aria-label="Storage folders">
        {focusedFiles && (
          <Button
            onClick={() => {
              setFocusedFiles(null);
              setSelection(null);
            }}
          >
            Clear group selection
          </Button>
        )}
        <Button
          onClick={() => {
            setFocusedFiles(null);
            setTrail([]);
            setSelection(null);
          }}
          aria-current={!trail.length ? 'location' : undefined}
        >
          Selected scope
        </Button>
        {trail.map((part, index) => (
          <Button
            key={JSON.stringify(trail.slice(0, index + 1))}
            aria-current={index === trail.length - 1 ? 'location' : undefined}
            onClick={() => {
              setFocusedFiles(null);
              setTrail(trail.slice(0, index + 1));
              setSelection(null);
            }}
          >
            {index === 0
              ? (roots.find((root) => root.id === part)?.label ?? 'Registered root')
              : part}
          </Button>
        ))}
        <Button
          disabled={!trail.length}
          onClick={() => {
            setFocusedFiles(null);
            setTrail(trail.slice(0, -1));
            setSelection(null);
          }}
        >
          Parent folder
        </Button>
      </chakra.nav>
      <Box className="storage-legend" aria-label={`${mode} map legend`}>
        {legend.map((label) => {
          const file = page.rows.find((file) =>
            mode === 'Age'
              ? ageOf(file) === label
              : mode === 'Camera'
                ? cameraOf(file, cameras) === label
                : categoryOf(file) === label,
          );
          return (
            <Text as="span" key={label}>
              <i style={{ background: file ? colorOf(file, mode, cameras) : 'var(--muted)' }} />{' '}
              {label}
            </Text>
          );
        })}
      </Box>
      <Box className="storage-workspace">
        <Box>
          {!filtered.length ? (
            <Box className="storage-empty" role="status">
              <Heading as="h3" unstyled>
                No files match this view
              </Heading>
              <Text>Clear a filter or load more observations.</Text>
              <Button onClick={reset}>Clear storage filters</Button>
            </Box>
          ) : view === 'Map' ? (
            <>
              <StorageMap
                groups={mapGroups}
                mode={mode}
                cameras={cameras}
                selected={selectedFile?.id ?? null}
                select={setSelection}
                selectGroup={setSelection}
                openGroup={openGroup}
              />
              <Text className="storage-note">
                Area represents logical size. Empty files have no area; use List to find them.
                Select a folder heading, then Open folder, or double-click it.
              </Text>
            </>
          ) : (
            <chakra.section aria-label="Inspected files">
              <Heading as="h3" unstyled>
                Files · largest first
              </Heading>
              <Table.Root unstyled className="storage-table">
                <Table.Header unstyled>
                  <Table.Row unstyled>
                    <Table.ColumnHeader unstyled>File / location</Table.ColumnHeader>
                    <Table.ColumnHeader unstyled>Size</Table.ColumnHeader>
                    <Table.ColumnHeader unstyled>File type</Table.ColumnHeader>
                  </Table.Row>
                </Table.Header>
                <Table.Body unstyled>
                  {sorted.map((file) => (
                    <Table.Row unstyled key={file.id}>
                      <Table.ColumnHeader unstyled scope="row">
                        <Button
                          aria-pressed={selectedFile?.id === file.id}
                          onClick={() => setSelection(file)}
                        >
                          {file.name}
                        </Button>
                        <small>{file.displayPath}</small>
                      </Table.ColumnHeader>
                      <Table.Cell unstyled data-bytes={file.bytes}>
                        {displayBytes(file.bytes)}
                      </Table.Cell>
                      <Table.Cell unstyled>{categoryOf(file)}</Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table.Root>
            </chakra.section>
          )}
          <details onToggle={(event) => setFolderTotalsOpen(event.currentTarget.open)}>
            <summary>Complete retained folder totals</summary>
            {folderTotalsOpen && <StorageReview jobId={jobId} summary={summary} />}
          </details>
        </Box>
        <chakra.aside className="storage-inspector" aria-label="File inspector">
          {!selection && (
            <>
              <Heading as="h3" unstyled>
                File inspector
              </Heading>
              <Text>Select a file or folder to inspect its size and location.</Text>
            </>
          )}
          {selectedFile && (
            <>
              <Box
                className="storage-file-icon"
                aria-label={`${categoryOf(selectedFile)} file icon`}
              >
                ▤
              </Box>
              <Heading as="h3" unstyled>
                {selectedFile.name}
              </Heading>
              <dl>
                <dt>Location</dt>
                <dd>{selectedFile.displayPath}</dd>
                <dt>Size</dt>
                <dd>{displayBytes(selectedFile.bytes)}</dd>
                <dt>Share of current view</dt>
                <dd>
                  {filtered.some((file) => file.id === selectedFile.id)
                    ? percentage(selectedFile.bytes, bytes)
                    : 'Outside current view'}
                </dd>
                <dt>File type</dt>
                <dd>
                  {categoryOf(selectedFile)}
                  {selectedFile.format ? ` · ${selectedFile.format}` : ''}
                </dd>
                <dt>Last modified</dt>
                <dd>
                  {selectedFile.filesystemModified &&
                  Number.isFinite(Date.parse(selectedFile.filesystemModified))
                    ? new Date(selectedFile.filesystemModified).toLocaleString()
                    : 'Unknown date'}
                </dd>
                <dt>Camera make / model</dt>
                <dd>{selectedFile.isMedia ? cameraOf(selectedFile, cameras) : 'Not applicable'}</dd>
                <dt>Availability</dt>
                <dd>{selectedFile.inventory.replaceAll('_', ' ')}</dd>
              </dl>
              {selectedFile.error && <Text role="status">{selectedFile.error.message}</Text>}
              <Button
                disabled={!!protectedFile || tray.some((file) => file.id === selectedFile.id)}
                onClick={() => setTray((previous) => [...previous, selectedFile])}
              >
                Add to review tray
              </Button>
              {protectedFile && (
                <Text className="storage-note">
                  Protected, read-only or protection status unknown. Review selection is
                  unavailable.
                </Text>
              )}
            </>
          )}
          {selectedGroup && (
            <>
              <Heading as="h3" unstyled>
                {selectedGroup.name}
              </Heading>
              <Text>
                {selectedGroup.files.length} files · {displayBytes(totalBytes(selectedGroup.files))}
              </Text>
              <dl>
                {categories.map((category) => {
                  const entries = selectedGroup.files.filter(
                    (file) => categoryOf(file) === category,
                  );
                  return (
                    entries.length > 0 && (
                      <Box key={category}>
                        <dt>{category}</dt>
                        <dd>{displayBytes(totalBytes(entries))}</dd>
                      </Box>
                    )
                  );
                })}
              </dl>
              <Button onClick={() => openGroup(selectedGroup)}>
                {selectedGroup.path ? 'Open folder' : 'Open group in list'}
              </Button>
            </>
          )}
          <chakra.section className="storage-tray" aria-label="Review tray">
            <Heading as="h3" unstyled>
              Review tray · {tray.length}
            </Heading>
            <Text className="storage-note">Planning only. No files are moved or deleted.</Text>
            {tray.map((file) => (
              <Text key={file.id}>
                {file.name} · {displayBytes(file.bytes)}{' '}
                <Button
                  aria-label={`Remove ${file.name} from tray`}
                  onClick={() =>
                    setTray((previous) => previous.filter((item) => item.id !== file.id))
                  }
                >
                  Remove
                </Button>
              </Text>
            ))}
            <Button
              disabled={!tray.length}
              aria-pressed={simulate}
              onClick={() => {
                setSimulate((value) => !value);
                setSelection(null);
              }}
            >
              {simulate ? 'End simulation' : 'Preview without these files'}
            </Button>
            {simulate && (
              <Text role="status">
                Simulation: {displayBytes(totalBytes(tray))} of logical file sizes excluded. This is
                not guaranteed reclaimed disk space. Drive totals are unchanged.
              </Text>
            )}
          </chakra.section>
        </chakra.aside>
      </Box>
    </Box>
  );
}
