import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useEffect, useRef, useState } from 'react';
import type { CopyWorkflowJob, CopyIoStatus, CopyJobState } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { copyJobActive } from '../../library/copyJobs';

const labels: Record<CopyJobState, string> = {
  queued: 'Queued',
  running: 'Running',
  pause_requested: 'I/O pause requested',
  cancel_requested: 'Cancellation requested',
  prepared: 'Output names prepared',
  completed_verified: 'File bytes copied and verified',
  canceled_with_partial_copies: 'Canceled; output objects retained',
  needs_attention: 'Needs attention',
  needs_recovery: 'Recovery inspection required',
};
export function CopyJobProgress({
  job,
  onRefresh,
}: {
  job: CopyWorkflowJob;
  onRefresh?: () => Promise<void>;
}) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  async function cancel() {
    setPending(true);
    setError(null);
    try {
      await libraryData(
        { kind: 'organization_copy_cancel', job_id: job.id },
        'organization_copy_job',
      );
      await onRefresh?.();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
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
      aria-label={job.action === 'prepare_names' ? 'Output preparation job' : 'Verified copy job'}
    >
      <Heading as="h3" unstyled>
        {labels[job.state]}
      </Heading>
      <Text>
        {job.action === 'prepare_names'
          ? 'Preparing native names; no source bytes copied.'
          : `${job.verifiedFiles} of ${job.totalFiles} files byte-verified.`}
      </Text>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        {job.message}
      </Text>
      {job.error && (
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
          {job.error.message}
        </Text>
      )}
      {copyJobActive(job) && (
        <Button
          type="button"
          disabled={pending || job.state === 'cancel_requested'}
          onClick={() => void cancel()}
        >
          {pending || job.state === 'cancel_requested' ? 'Stopping…' : 'Cancel organization job'}
        </Button>
      )}
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
    </chakra.section>
  );
}
export function CopyIoControls() {
  const [status, setStatus] = useState<CopyIoStatus | null>(null);
  const [rate, setRate] = useState('10240');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(() => {
    const token = ++epoch.current;
    libraryData({ kind: 'organization_copy_io' }, 'organization_copy_io')
      .then((value) => {
        if (epoch.current === token) {
          setStatus(value);
          setRate((BigInt(value.aggregateBytesPerSecond) / 1024n).toString());
        }
      })
      .catch((error) => {
        if (epoch.current === token) setError(errorMessage(error));
      });
    return () => {
      epoch.current += 1;
    };
  }, []);
  async function update(paused: boolean | null) {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      const bytes = paused === null ? (BigInt(rate) * 1024n).toString() : null;
      const value = await libraryData(
        {
          kind: 'organization_copy_set_io',
          aggregate_bytes_per_second: bytes,
          byte_io_paused: paused,
        },
        'organization_copy_io',
      );
      if (epoch.current === token) setStatus(value);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <chakra.section aria-label="Operation I/O controls" aria-busy={pending}>
      <Heading as="h4" unstyled>
        Operation I/O
      </Heading>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Aggregate KiB/s </span>
          </Field.Label>
          <Input
            type="number"
            min="1"
            max="1048576"
            step="1"
            value={rate}
            onChange={(event) => setRate(event.target.value)}
            disabled={pending}
          />
        </label>
      </Field.Root>
      <Button type="button" disabled={pending || !status} onClick={() => void update(null)}>
        Apply I/O limit
      </Button>
      <Button
        type="button"
        disabled={pending || !status}
        onClick={() => void update(!status?.byteIoPaused)}
      >
        {status?.byteIoPaused ? 'Resume byte I/O' : 'Pause byte I/O'}
      </Button>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Reads, writes and verification share this limit. Pause takes effect at the next admitted
        byte boundary. Metadata, flushes and external helpers have separate limits.
      </Text>
      {status && (
        <Text>
          {BigInt(status.aggregateBytesPerSecond).toLocaleString()} bytes/s ·{' '}
          {status.byteIoPaused ? 'byte I/O pause requested' : 'byte I/O enabled'}
        </Text>
      )}
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
    </chakra.section>
  );
}
