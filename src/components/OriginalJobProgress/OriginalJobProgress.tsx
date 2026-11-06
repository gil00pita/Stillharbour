import type { OriginalOperationJob } from '../../bindings';

export function OriginalJobProgress({ job }: { job: OriginalOperationJob }) {
  const running = job.state === 'queued' || job.state === 'running';
  const completed = job.state === 'completed_verified';
  return (
    <section
      className="job-entry"
      aria-label={`${job.kind === 'undo' ? 'Undo' : 'Rename'} organization job`}
    >
      <h3>{job.kind === 'undo' ? 'Undo organization' : 'Rename files'} · Generated fixture</h3>
      <p>
        {job.state === 'needs_recovery'
          ? 'Needs recovery review'
          : completed
            ? 'Completed and verified'
            : job.state === 'queued'
              ? 'Queued'
              : 'Running'}
      </p>
      {running ? (
        <progress aria-label="Organization job in progress" />
      ) : (
        <progress aria-label="Organization job outcome" max={1} value={completed ? 1 : 0} />
      )}
      <p>
        {job.verifiedMoves} / {job.totalMoves} verified moves
      </p>
      <p className="help">{job.message}</p>
    </section>
  );
}
