import { render, screen } from '../../test/render';
import { expect, it } from 'vitest';
import type { OriginalOperationJob } from '../../bindings';
import { OriginalJobProgress } from '@/components/OriginalJobProgress';

function job(state: OriginalOperationJob['state']): OriginalOperationJob {
  return {
    id: 'job',
    planId: 'plan',
    sourceRootId: 'root',
    kind: 'rename',
    state,
    totalMoves: 10,
    verifiedMoves: state === 'completed_verified' ? 10 : 0,
    startedAt: 'now',
    updatedAt: 'now',
    message: 'Retained scope only; no approval was restored.',
    generatedFixtureOnly: true,
  };
}
it('keeps queued and running movement indeterminate until verification commits', () => {
  const view = render(<OriginalJobProgress job={job('queued')} />);
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('value');
  view.rerender(<OriginalJobProgress job={job('running')} />);
  expect(screen.getByRole('progressbar')).not.toHaveAttribute('value');
  expect(screen.getByText('0 / 10 verified moves')).toBeInTheDocument();
});
it('fills a completed job only from the backend verified terminal state', () => {
  render(<OriginalJobProgress job={job('completed_verified')} />);
  expect(screen.getByRole('progressbar')).toHaveAttribute('value', '1');
  expect(screen.getByText('Completed and verified')).toBeInTheDocument();
});
it('shows interrupted jobs as stopped recovery work without a replay action', () => {
  render(<OriginalJobProgress job={{ ...job('needs_recovery'), kind: 'undo' }} />);
  expect(screen.getByText('Undo organization · Generated fixture')).toBeInTheDocument();
  expect(screen.getByText('Needs recovery review')).toBeInTheDocument();
  expect(screen.getByRole('progressbar')).toHaveAttribute('value', '0');
  expect(screen.queryByRole('button')).not.toBeInTheDocument();
});
