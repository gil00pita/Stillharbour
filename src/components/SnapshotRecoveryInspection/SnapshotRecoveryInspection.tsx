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
    <chakra.section aria-label="Inspect interrupted snapshot" className="backup-review">
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
