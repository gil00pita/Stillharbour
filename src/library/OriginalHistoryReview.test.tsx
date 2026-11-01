import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { OriginalOperationHistory, OriginalOperationJob } from '../bindings';
import { libraryData } from '../ipc';
import { OriginalHistoryReview } from './OriginalHistoryReview';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const job: OriginalOperationJob = {
  id: 'job',
  planId: 'plan',
  sourceRootId: 'root',
  kind: 'rename',
  state: 'needs_recovery',
  totalMoves: 2,
  verifiedMoves: 0,
  startedAt: 'now',
  updatedAt: 'now',
  message: 'Stopped',
  generatedFixtureOnly: true,
};
function result(patch: Partial<OriginalOperationHistory> = {}): OriginalOperationHistory {
  return {
    job,
    planDigest: 'a'.repeat(64),
    reverseOf: null,
    members: [{ id: 'member', before: '@name.JPG', after: 'renamed.JPG' }],
    historyOnly: true,
    currentLocationsChecked: false,
    executionAvailable: false,
    ...patch,
  };
}
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value() {
      this.setAttribute('open', '');
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value() {
      this.removeAttribute('open');
    },
  });
});
it('opens explicit retained history without picker, scan or mutation and keeps paths as text', async () => {
  vi.mocked(libraryData).mockResolvedValue(result());
  render(<OriginalHistoryReview job={job} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  expect(await screen.findByText('@name.JPG')).toBeInTheDocument();
  expect(screen.getByText('Needs recovery review')).toBeInTheDocument();
  expect(screen.getByText(/Current file locations and contents require/)).toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledExactlyOnceWith(
    { kind: 'organization_original_history', job_id: 'job' },
    'organization_original_history',
  );
  expect(
    screen.queryByRole('button', { name: /resume|approve|run|undo|restore/i }),
  ).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Close plan history' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
it('rejects a different job and unsafe capability flags, with explicit retry', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(result({ job: { ...job, id: 'different' } }))
    .mockResolvedValueOnce(result({ executionAvailable: true }))
    .mockResolvedValueOnce(result());
  render(<OriginalHistoryReview job={job} />);
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('does not match');
  fireEvent.click(screen.getByRole('button', { name: 'Retry history' }));
  await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(2));
  expect(await screen.findByRole('alert')).toHaveTextContent('does not match');
  expect(screen.queryByText('@name.JPG')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Retry history' }));
  expect(await screen.findByText('@name.JPG')).toBeInTheDocument();
});
it('discards a response after closing and bounds the visible member table', async () => {
  let resolve!: (value: OriginalOperationHistory) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  render(<OriginalHistoryReview job={job} />);
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  fireEvent.click(screen.getByRole('button', { name: 'Close plan history' }));
  await act(async () => resolve(result()));
  expect(screen.queryByText('@name.JPG')).not.toBeInTheDocument();
  vi.mocked(libraryData).mockResolvedValueOnce(
    result({
      members: Array.from({ length: 65 }, (_, i) => ({
        id: `id-${i}`,
        before: `before-${i}.JPG`,
        after: `after-${i}.JPG`,
      })),
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  expect(await screen.findByText('before-0.JPG')).toBeInTheDocument();
  expect(screen.queryByText('before-64.JPG')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Next members' }));
  expect(screen.getByText('before-64.JPG')).toBeInTheDocument();
  expect(screen.queryByText('before-0.JPG')).not.toBeInTheDocument();
});

it('retires a pending history read when the job identity changes', async () => {
  let resolve!: (value: OriginalOperationHistory) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const view = render(<OriginalHistoryReview job={job} />);
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  const nextJob = { ...job, id: 'next-job', planId: 'next-plan' };
  view.rerender(<OriginalHistoryReview job={nextJob} />);
  await act(async () => resolve(result()));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  vi.mocked(libraryData).mockResolvedValueOnce(result({ job: nextJob }));
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  expect(await screen.findByText('@name.JPG')).toBeInTheDocument();
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_original_history', job_id: 'next-job' },
    'organization_original_history',
  );
});

it('closes a nested history dialog without canceling its parent review', async () => {
  const parentCancel = vi.fn();
  vi.mocked(libraryData).mockResolvedValue(result());
  render(
    <dialog open aria-label="Parent fixture review" onCancel={parentCancel}>
      <OriginalHistoryReview job={job} />
    </dialog>,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Review retained organization plan' }));
  await screen.findByText('@name.JPG');
  fireEvent(
    screen.getByRole('dialog', { name: 'Retained organization plan' }),
    new Event('cancel', { bubbles: true, cancelable: true }),
  );
  expect(parentCancel).not.toHaveBeenCalled();
  expect(
    screen.queryByRole('dialog', { name: 'Retained organization plan' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('dialog', { name: 'Parent fixture review' })).toBeInTheDocument();
});
