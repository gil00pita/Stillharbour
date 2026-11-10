import { List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  CopyRecoveryInspection,
  CopyWorkflowJob,
  CopyInspectionState,
  DirectorySelection,
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';
import { CopyContinuationWorkflow } from '@/components/CopyContinuationWorkflow';

const labels: Record<CopyInspectionState, string> = {
  not_started: 'Not started',
  prepared_empty_stage: 'Original empty stage retained',
  retained_partial_stage: 'Partial stage retained',
  staged_verified: 'Staged bytes freshly verified',
  published_verified: 'Published bytes freshly verified',
  needs_attention: 'Needs attention; objects retained',
};

export function RetainedCopyInspection({
  job,
  onRefresh,
}: {
  job: CopyWorkflowJob;
  onRefresh?: () => Promise<void>;
}) {
  const [result, setResult] = useState<CopyRecoveryInspection | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [selection, setSelection] = useState<DirectorySelection | null>(null);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [job.id],
  );
  async function inspect() {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    setResult(null);
    setSelection(null);
    try {
      const capability = await libraryData(
        { kind: 'organization_copy_capabilities' },
        'organization_copy_capabilities',
      );
      if (epoch.current !== token) return;
      if (!capability.available) throw new Error(capability.reason);
      const selected = await pickDirectory();
      if (epoch.current !== token || !selected) return;
      const value = await libraryData(
        { kind: 'organization_copy_recovery_inspect', job_id: job.id, selection_id: selected.id },
        'organization_copy_recovery_inspection',
      );
      if (epoch.current !== token) return;
      if (
        value.jobId !== job.id ||
        value.sourceRootId !== job.sourceRootId ||
        value.files.length !== job.totalFiles ||
        value.inspection.states.length !== value.files.length ||
        value.inspection.reviewId !==
          (job.action === 'copy_files' ? job.reviewId : job.readyReviewId) ||
        value.files.some((file) => !file.targetPath.startsWith(`${value.outputFolder}/`))
      ) {
        throw new Error('The retained inspection does not match this job. Reopen its history.');
      }
      setPage(0);
      setResult(value);
      setSelection(selected);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <chakra.section aria-label="Retained copy inspection" aria-busy={pending}>
      <Button type="button" disabled={pending} onClick={() => void inspect()}>
        {pending ? 'Inspecting retained bytes…' : 'Choose original destination and inspect'}
      </Button>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Inspection compares actual files with their sources. It grants no permission to resume or
        change files. Originals and retained outputs remain in place.
      </Text>
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
      {result && (
        <>
          <Text>
            <bdi>{result.destinationDisplayPath}</bdi> / <bdi>{result.outputFolder}</bdi>
          </Text>
          <Text role="status">
            {result.inspection.allPublishedBytesVerified
              ? 'Every selected output was freshly compared with its source.'
              : 'Some files need attention or remain staged. Keep their files and journal for recovery.'}
          </Text>
          <List.Root asChild unstyled>
            <ol start={page * 64 + 1}>
              {result.files.slice(page * 64, (page + 1) * 64).map((file, index) => (
                <List.Item unstyled key={file.id}>
                  <Text>
                    <bdi>{file.sourcePath}</bdi> → <bdi>{file.targetPath}</bdi>
                  </Text>
                  <Text>
                    {file.bytes} bytes —{' '}
                    {labels[result.inspection.states[page * 64 + index] ?? 'needs_attention']}
                  </Text>
                  {result.issues
                    .filter((issue) => issue.fileId === file.id)
                    .map((issue) => (
                      <Text
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
                        key={issue.fileId}
                      >
                        {issue.error.code}: {issue.error.message}
                      </Text>
                    ))}
                </List.Item>
              ))}
            </ol>
          </List.Root>
          <Button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>
            Previous inspected files
          </Button>
          <Button
            type="button"
            disabled={(page + 1) * 64 >= result.files.length}
            onClick={() => setPage(page + 1)}
          >
            Next inspected files
          </Button>
          {selection && (
            <CopyContinuationWorkflow
              key={selection.id}
              job={job}
              selection={selection}
              {...(onRefresh ? { onRefresh } : {})}
            />
          )}
        </>
      )}
    </chakra.section>
  );
}
