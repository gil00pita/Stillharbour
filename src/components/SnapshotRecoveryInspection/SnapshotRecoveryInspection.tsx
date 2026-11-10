import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useState } from 'react';
import type { SnapshotJob } from '../../bindings';

export function SnapshotRecoveryInspection({
  job,
  disabled,
  onInspect,
}: {
  job: SnapshotJob;
  disabled: boolean;
  onInspect: (snapshotId: string | null) => void;
}) {
  const [snapshotId, setSnapshotId] = useState(job.snapshotId ?? '');
  const valid = !snapshotId || /^[a-fA-F0-9]{64}$/.test(snapshotId);
  return (
    <chakra.section
      aria-label="Inspect interrupted snapshot"
      className="backup-review"
      css={{
        border: '1px solid {colors.border}',
        borderRadius: '8px',
        padding: '20px',
        marginBlock: '20px',
        minWidth: '0',
        background: '{colors.bg.panel}',
        '& fieldset': {
          display: 'grid',
          gap: '16px',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          padding: '16px',
          marginBlock: '20px',
        },
        '& legend': { fontWeight: '600', paddingInline: '6px' },
        '& label': { display: 'grid', gap: '8px', minWidth: '0' },
        "& label:has(> input[type='checkbox'])": {
          display: 'flex',
          alignItems: 'flex-start',
          gap: '8px',
        },
        "& input[type='checkbox']": { flex: '0 0 auto', marginTop: '4px' },
        "& input:not([type='checkbox'])": {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
        },
        '& textarea': {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
          resize: 'vertical',
          minHeight: '100px',
        },
        '& select': {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
        },
        '& .backup-form-grid': { gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' },
        '& small': { color: '{colors.fg.muted}' },
        '& .backup-table': { maxHeight: '360px' },
        '@media (max-width: 850px)': { '& .backup-form-grid': { gridTemplateColumns: '1fr' } },
      }}
    >
      <Heading as="h5" unstyled>
        Inspect existing snapshot
      </Heading>
      <Text>
        Check the repository's existing content and compare a fresh restore with this job's frozen
        manifest. This does not retry the backup, remove a lock or delete history.
      </Text>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Existing snapshot ID (optional)</span>
          </Field.Label>
          <Input
            value={snapshotId}
            disabled={disabled}
            maxLength={64}
            spellCheck={false}
            onChange={(event) => setSnapshotId(event.target.value)}
          />
        </label>
      </Field.Root>
      <Text>
        Leave empty to inspect this job's recorded snapshot. If the result is ambiguous, use the
        exact 64-character snapshot ID reported by the backend.
      </Text>
      {!valid && (
        <Text role="status">
          A concrete snapshot ID must contain exactly 64 hexadecimal characters.
        </Text>
      )}
      <Button disabled={disabled || !valid} onClick={() => onInspect(snapshotId || null)}>
        Inspect existing snapshot
      </Button>
    </chakra.section>
  );
}
