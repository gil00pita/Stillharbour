import { Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { CopyManifest, ImportComparisonJob, LibraryRoot } from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { CopyIoControls } from '@/components/CopyJobProgress';
import { ImportPlanReview } from '@/components/ImportPlanReview';
import { importSourceScope } from '../../library/importPlanValidation';
import { RootPartitionCopyWorkflow } from '@/components/RootPartitionCopyWorkflow';

const active = (job: ImportComparisonJob) =>
  ['queued', 'running', 'cancel_requested'].includes(job.state);
const labels = {
  no_indexed_match: 'No match in the indexed scope',
  exact_bytes_compared: 'Complete bytes compared equal',
  same_native_object: 'Same native file object; no independent copy',
  needs_attention: 'Unknown; review comparison errors',
};
function validate(job: ImportComparisonJob, manifest: CopyManifest, roots: string[], id?: string) {
  if (
    (id && job.id !== id) ||
    job.planId !== manifest.planId ||
    job.manifestId !== manifest.id ||
    job.sourceRootId !== manifest.files[0]?.rootId ||
    JSON.stringify([...job.scopeRootIds].sort()) !== JSON.stringify([...roots].sort()) ||
    job.totalSourceFiles !== manifest.files.length ||
    job.comparedSourceFiles > job.totalSourceFiles ||
    job.candidateFiles > 4096 ||
    !/^\d{1,39}$/.test(job.bytesRead)
  )
    throw new Error('The comparison belongs to another source or library scope.');
  if (job.report) {
    const report = job.report;
    if (
      report.policy !== 'indexed-import-comparison-v1' ||
      report.algorithm !== 'blake3' ||
      report.samplePolicy !== 'import-three-window-blake3-v1' ||
      report.indexedPresentFilesOnly !== true ||
      report.bundleReviewRequired !== true ||
      report.mutationAuthorized !== false ||
      !['completed', 'completed_with_issues'].includes(job.state) ||
      (job.state === 'completed') !== report.allScopeAvailable ||
      report.files.length !== manifest.files.length ||
      new Set(report.files.map((file) => file.fileId)).size !== manifest.files.length ||
      report.bundles.length === 0 ||
      report.bundles.length > manifest.files.length ||
      report.bundles.flatMap((bundle) => bundle.fileIds).length !== manifest.files.length ||
      new Set(report.bundles.flatMap((bundle) => bundle.fileIds)).size !== manifest.files.length ||
      report.bundles.some((bundle) => {
        const members = manifest.files.filter((file) => bundle.fileIds.includes(file.id));
        const ids = [...new Set(members.flatMap((file) => file.bundleAssetIds))].sort();
        return (
          !members.length ||
          members.length !== bundle.fileIds.length ||
          JSON.stringify(ids) !== JSON.stringify(bundle.bundleAssetIds) ||
          manifest.files.some(
            (file) =>
              file.bundleAssetIds.some((id) => ids.includes(id)) &&
              !bundle.fileIds.includes(file.id),
          ) ||
          !['copy_whole_bundle', 'review_existing_bundle', 'needs_attention'].includes(
            bundle.disposition,
          ) ||
          (bundle.disposition === 'review_existing_bundle') !== bundle.existingGroups.length > 0 ||
          (!report.allScopeAvailable && bundle.disposition !== 'needs_attention') ||
          bundle.existingGroups.length > 4 ||
          bundle.existingGroups.some(
            (group) =>
              !/^[a-f0-9]{64}$/.test(group.id) ||
              !roots.includes(group.rootId) ||
              group.members.length !== members.length ||
              new Set(group.members.map((member) => member.fileId)).size !== members.length ||
              new Set(group.members.map((member) => member.assetId)).size !== members.length ||
              group.members.some(
                (member) =>
                  !bundle.fileIds.includes(member.fileId) ||
                  !report.files
                    .find((file) => file.fileId === member.fileId)
                    ?.matches.some(
                      (match) => match.assetId === member.assetId && match.rootId === group.rootId,
                    ),
              ),
          )
        );
      }) ||
      report.files.some((file) => {
        const source = manifest.files.find((source) => source.id === file.fileId);
        return (
          !source ||
          !Object.hasOwn(labels, file.state) ||
          ['exact_bytes_compared', 'same_native_object'].includes(file.state) !==
            file.matches.length > 0 ||
          source.bytes !== file.bytes ||
          file.matches.length > 4 ||
          file.matches.some(
            (match) => !roots.includes(match.rootId) || !/^[a-f0-9]{64}$/.test(match.digest),
          )
        );
      })
    )
      throw new Error('The comparison report does not match the selected files.');
  }
}

export function ImportComparisonReview({ manifest }: { manifest: CopyManifest }) {
  if (new Set(manifest.files.map((file) => file.rootId)).size > 1)
    return (
      <RootPartitionCopyWorkflow key={manifest.id} manifest={manifest} purpose="import">
        {(root, activity) => (
          <ImportComparisonSession
            key={root}
            completeManifest={manifest}
            sourceRootId={root}
            onActivityChange={activity}
          />
        )}
      </RootPartitionCopyWorkflow>
    );
  return <ImportComparisonSession completeManifest={manifest} />;
}
function ImportComparisonSession({
  completeManifest,
  sourceRootId,
  onActivityChange,
}: {
  completeManifest: CopyManifest;
  sourceRootId?: string;
  onActivityChange?: (active: boolean) => void;
}) {
  const manifest = useMemo(
    () => (sourceRootId ? importSourceScope(completeManifest, sourceRootId) : completeManifest),
    [completeManifest, sourceRootId],
  );
  const [roots, setRoots] = useState<LibraryRoot[] | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [job, setJob] = useState<ImportComparisonJob | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [importActive, setImportActive] = useState(false);
  const epoch = useRef(0);
  const retained = useRef<string | null>(null);
  const expectedScope = useRef<string[]>([]);
  const jobId = job?.id;
  const heading = useRef<HTMLHeadingElement>(null);
  const running = job !== null && active(job);
  useEffect(() => {
    onActivityChange?.(pending || job !== null);
    return () => onActivityChange?.(false);
  }, [pending, job, onActivityChange]);
  const sourceRoots = new Set(manifest.files.map((file) => file.rootId));
  useEffect(
    () => () => {
      epoch.current += 1;
      if (retained.current)
        void library({
          kind: 'organization_import_comparison_discard',
          job_id: retained.current,
        }).catch(() => {});
    },
    [],
  );
  useEffect(() => {
    if (job?.report) heading.current?.focus();
  }, [job?.report]);
  useEffect(() => {
    if (!jobId || !running) return;
    const id = jobId;
    const scope = expectedScope.current;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const current = await libraryData(
          { kind: 'organization_import_comparison_job', job_id: id },
          'organization_import_comparison_job',
        );
        if (!live) return;
        validate(current, manifest, scope, id);
        setJob(current);
        if (active(current)) timer = setTimeout(() => void poll(), 250);
      } catch (error) {
        if (live) {
          setError(errorMessage(error));
          timer = setTimeout(() => void poll(), 1000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 100);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [jobId, running, manifest]);
  async function open() {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      const capability = await libraryData(
        { kind: 'organization_copy_capabilities' },
        'organization_copy_capabilities',
      );
      if (epoch.current !== token) return;
      if (!capability.available) throw new Error('Import comparison is not enabled in this build.');
      const status = await libraryData({ kind: 'status' }, 'status');
      if (epoch.current === token) setRoots(status.roots);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  async function start() {
    if (pending || running || importActive) return;
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    setPage(0);
    try {
      if (retained.current)
        await library({
          kind: 'organization_import_comparison_discard',
          job_id: retained.current,
        }).catch(() => {});
      retained.current = null;
      setJob(null);
      const scope = [...selected];
      const value = await libraryData(
        {
          kind: 'organization_import_comparison_start',
          input: {
            planId: manifest.planId,
            manifestId: manifest.id,
            scopeRootIds: scope,
            ...(sourceRootId ? { sourceRootId } : {}),
          },
        },
        'organization_import_comparison_job',
      );
      if (epoch.current !== token) {
        if (value.planId === manifest.planId && value.manifestId === manifest.id)
          void library({ kind: 'organization_import_comparison_discard', job_id: value.id }).catch(
            () => {},
          );
        return;
      }
      validate(value, manifest, scope);
      retained.current = value.id;
      expectedScope.current = scope;
      setJob(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  async function cancel() {
    if (!job) return;
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      const value = await libraryData(
        { kind: 'organization_import_comparison_cancel', job_id: job.id },
        'organization_import_comparison_job',
      );
      if (epoch.current !== token) return;
      validate(value, manifest, job.scopeRootIds, job.id);
      setJob(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  function selectRoot(id: string, checked: boolean) {
    if (pending || running || importActive) return;
    epoch.current += 1;
    if (retained.current)
      void library({
        kind: 'organization_import_comparison_discard',
        job_id: retained.current,
      }).catch(() => {});
    retained.current = null;
    setJob(null);
    setPage(0);
    setError(null);
    setSelected((old) => (checked ? [...old, id] : old.filter((root) => root !== id)));
  }
  const report = job?.report;
  async function closeReport() {
    if (!job || pending || running || importActive) return;
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      await library({ kind: 'organization_import_comparison_discard', job_id: job.id });
      if (epoch.current === token) {
        retained.current = null;
        setJob(null);
        setPage(0);
      }
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  const pages = Math.max(1, Math.ceil((report?.files.length ?? 0) / 64));
  return (
    <chakra.section aria-label="Import byte comparison" className="organization-destination">
      <Heading as="h3" unstyled>
        Compare before importing
      </Heading>
      <Text>
        Compare selected file bytes with an explicit indexed library scope. Size and sampled/full
        BLAKE3 hashes filter candidates; every reported match requires a complete direct byte
        comparison. This review grants no copy-skipping or removal permission.
      </Text>
      {!roots ? (
        <Button type="button" disabled={pending} onClick={() => void open()}>
          Choose indexed comparison scope
        </Button>
      ) : (
        <>
          <chakra.fieldset disabled={pending || running || importActive}>
            <chakra.legend>Existing indexed roots (choose 1–16)</chakra.legend>
            {roots.map((root) => (
              <label key={root.id}>
                <input
                  type="checkbox"
                  checked={selected.includes(root.id)}
                  disabled={sourceRoots.has(root.id)}
                  onChange={(event) => selectRoot(root.id, event.target.checked)}
                />
                {root.label} · {root.availability}
                {sourceRoots.has(root.id) ? ' · selected source' : ''}
              </label>
            ))}
          </chakra.fieldset>
          <Button
            type="button"
            disabled={
              pending ||
              running ||
              importActive ||
              !selected.length ||
              selected.length > 16 ||
              manifest.files.length > 512
            }
            onClick={() => void start()}
          >
            Start read-only comparison
          </Button>
          {manifest.files.length > 512 && (
            <Text>Compare at most 512 confirmed source files per review.</Text>
          )}
        </>
      )}
      {job && (
        <Text role="status">
          Comparison: {job.state.replaceAll('_', ' ')} · {job.stage} · {job.comparedSourceFiles}/
          {job.totalSourceFiles} selected files · {job.candidateFiles} admitted candidates ·{' '}
          {BigInt(job.bytesRead).toLocaleString()} payload bytes read.
        </Text>
      )}
      {job && !running && (
        <Button disabled={pending || importActive} onClick={() => void closeReport()}>
          Close comparison report
        </Button>
      )}
      {running && (
        <>
          <Button
            type="button"
            disabled={pending || job.state === 'cancel_requested'}
            onClick={() => void cancel()}
          >
            Cancel read-only comparison
          </Button>
          <CopyIoControls />
        </>
      )}
      {job?.state === 'canceled' && (
        <Text>Comparison canceled at a safe read boundary. Rerun to obtain a complete report.</Text>
      )}
      {(error || job?.error) && <Text role="alert">{error ?? errorMessage(job?.error)}</Text>}
      {report && (
        <>
          <Heading as="h4" unstyled ref={heading} tabIndex={-1}>
            Observed file comparison
          </Heading>
          <Text>
            {report.allScopeAvailable
              ? 'Selected indexed scope was accessible.'
              : 'Comparison coverage is incomplete; unknown files remain unknown.'}{' '}
            {report.excludedOrMissingFiles} missing or excluded indexed entries were omitted.
            Observed {report.observedAt}; this is not an atomic filesystem snapshot.
          </Text>
          <Text>
            Bundle observations keep confirmed companions together. No files were skipped, copied or
            removed by this comparison. Existing groups need fresh complete-byte verification and
            separate import approval. Reports must be rerun after restart.
          </Text>
          <details>
            <summary>Whole-bundle import observations ({report.bundles.length})</summary>
            {report.bundles.map((bundle) => (
              <Box key={bundle.fileIds[0]}>
                <Text>
                  {bundle.fileIds.length} selected members ·{' '}
                  {bundle.disposition === 'review_existing_bundle'
                    ? `${bundle.existingGroups.length} complete existing groups to review`
                    : bundle.disposition === 'copy_whole_bundle'
                      ? 'Copy the whole bundle'
                      : 'Bundle needs attention'}
                </Text>
                <Text>{bundle.reason}</Text>
                <List.Root unstyled>
                  {bundle.fileIds.map((id) => (
                    <List.Item unstyled key={id}>
                      <bdi>{manifest.files.find((file) => file.id === id)?.sourceName}</bdi>
                    </List.Item>
                  ))}
                </List.Root>
              </Box>
            ))}
          </details>
          <Box className="organization-table-scroll">
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top">
                Compared source files, page {page + 1} of {pages}
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled>Source relative path</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Result</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Existing indexed matches</Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {report.files.slice(page * 64, (page + 1) * 64).map((file) => (
                  <Table.Row unstyled key={file.fileId}>
                    <Table.Cell unstyled>
                      <bdi>{file.sourcePath}</bdi>
                    </Table.Cell>
                    <Table.Cell unstyled>{labels[file.state]}</Table.Cell>
                    <Table.Cell unstyled>
                      {file.matches.map((match) => (
                        <Text key={match.assetId}>
                          <bdi>{match.relativePath}</bdi>
                          {match.displayTruncated ? ' (display shortened)' : ''}
                          {match.sameNativeObject ? ' · same physical object' : ''}
                        </Text>
                      ))}
                      {file.furtherCandidatesUncompared && (
                        <Text>
                          Additional hash candidates were not directly compared (four-match limit).
                        </Text>
                      )}
                    </Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Button type="button" disabled={!page} onClick={() => setPage(page - 1)}>
            Previous comparison files
          </Button>
          <Button type="button" disabled={page + 1 >= pages} onClick={() => setPage(page + 1)}>
            Next comparison files
          </Button>
          {report.issues.length > 0 && (
            <List.Root unstyled aria-label="Comparison issues">
              {report.issues.map((issue, index) => (
                <List.Item unstyled key={index}>
                  <bdi>
                    {roots?.find((root) => root.id === issue.rootId)?.label ?? issue.rootId}
                  </bdi>
                  : {errorMessage(issue.error)}
                </List.Item>
              ))}
            </List.Root>
          )}
        </>
      )}
      <ImportPlanReview
        key={job?.id ?? 'history'}
        manifest={manifest}
        comparison={job ?? undefined}
        onActivityChange={setImportActive}
      />
    </chakra.section>
  );
}
