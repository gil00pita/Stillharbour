import { useState } from 'react';
import type { SnapshotJob } from '../bindings';

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
    <section aria-label="Inspect interrupted snapshot" className="backup-review">
      <h5>Inspect existing snapshot</h5>
      <p>
        Check the repository's existing content and compare a fresh restore with this job's frozen
        manifest. This does not retry the backup, remove a lock or delete history.
      </p>
      <label>
        Existing snapshot ID (optional)
        <input
          value={snapshotId}
          disabled={disabled}
          maxLength={64}
          spellCheck={false}
          onChange={(event) => setSnapshotId(event.target.value)}
        />
      </label>
      <p>
        Leave empty to inspect this job's recorded snapshot. If the result is ambiguous, use the
        exact 64-character snapshot ID reported by the backend.
      </p>
      {!valid && (
        <p role="status">A concrete snapshot ID must contain exactly 64 hexadecimal characters.</p>
      )}
      <button disabled={disabled || !valid} onClick={() => onInspect(snapshotId || null)}>
        Inspect existing snapshot
      </button>
    </section>
  );
}
