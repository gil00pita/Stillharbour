import { Heading, Text, chakra } from '@chakra-ui/react';
import type { OriginalOperationJob } from '../../bindings';

export function OriginalJobProgress({ job }: { job: OriginalOperationJob }) {
  const running = job.state === 'queued' || job.state === 'running';
  const completed = job.state === 'completed_verified';
  return (
    <chakra.section
      className="job-entry"
      aria-label={`${job.kind === 'undo' ? 'Undo' : 'Rename'} organization job`}
    >
      <Heading as="h3" unstyled>
        {job.kind === 'undo' ? 'Undo organization' : 'Rename files'} · Generated fixture
      </Heading>
      <Text>
        {job.state === 'needs_recovery'
          ? 'Needs recovery review'
          : completed
            ? 'Completed and verified'
            : job.state === 'queued'
              ? 'Queued'
              : 'Running'}
      </Text>
      {running ? (
        <progress aria-label="Organization job in progress" />
      ) : (
        <progress aria-label="Organization job outcome" max={1} value={completed ? 1 : 0} />
      )}
      <Text>
        {job.verifiedMoves} / {job.totalMoves} verified moves
      </Text>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        {job.message}
      </Text>
    </chakra.section>
  );
}
