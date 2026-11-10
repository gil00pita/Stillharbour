import { Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useState } from 'react';
import type { InspectionJob } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { displayBytes } from '../../inspection/format';
export function InspectionJobProgress({
  job,
  onRefresh,
}: {
  job: InspectionJob;
  onRefresh: () => Promise<void>;
}) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const active = ['queued', 'running', 'cancel_requested'].includes(job.state);
  async function cancel() {
    setBusy(true);
    setError(null);
    try {
      await libraryData({ kind: 'inspection_cancel', job_id: job.id }, 'inspection_job');
      await onRefresh();
    } catch (failure) {
      setError(errorMessage(failure));
    } finally {
      setBusy(false);
    }
  }
  return (
    <chakra.section
      className="job-row"
      css={{
        padding: '20px 0',
        borderBottom: '1px solid {colors.border}',
        '&:last-child': { borderBottom: '0', paddingBottom: '0' },
        '& p': { fontSize: '13px', lineHeight: '20px' },
      }}
      aria-label={`${job.input.task.kind} inspection job`}
    >
      <Heading as="h3" unstyled>
        {job.input.task.kind === 'duplicates'
          ? 'Duplicate'
          : job.input.task.kind === 'health'
            ? 'Integrity'
            : 'Storage'}{' '}
        inspection · {job.state.replaceAll('_', ' ')}
      </Heading>
      <Text>{job.message}</Text>
      <Text>
        {job.filesObserved} observed · {job.filesChecked} examined · {displayBytes(job.bytesRead)}{' '}
        read · {job.issues} issues
      </Text>
      {active && (
        <>
          <progress aria-label="Inspection in progress" />
          <Button disabled={busy || job.state === 'cancel_requested'} onClick={() => void cancel()}>
            Cancel inspection
          </Button>
        </>
      )}
      <Text>Open Duplicates, Integrity or Storage to review the retained scope and results.</Text>
      {error && <Text role="alert">{error}</Text>}
    </chakra.section>
  );
}
