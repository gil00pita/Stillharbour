import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  GeneratedRenameReview,
  GeneratedWorkerStatus,
  OriginalOperationJob,
} from './bindings';
import { GeneratedRenameLab } from './GeneratedRenameLab';

const { requestInvoke, workerInvoke } = vi.hoisted(() => ({
  requestInvoke: vi.fn(),
  workerInvoke: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (command: string, args: unknown) =>
    command === 'test_generated_rename_worker'
      ? workerInvoke(command, args)
      : requestInvoke(command, args),
}));
function review(patch: Partial<GeneratedRenameReview> = {}): GeneratedRenameReview {
  return {
    planId: 'forward',
    digest: 'digest',
    reverse: false,
    state: 'review',
    completedSteps: 0,
    totalSteps: 10,
    pendingIntent: false,
    catalogCommitted: patch.state === 'completed',
    jobs: [],
    members: [
      {
        id: 'member',
        before: 'Case.JPG',
        after: 'case.JPG',
        observed: 'Case.JPG',
        catalogAssetId: 'stable-asset',
        catalogPath: 'Case.JPG',
      },
    ],
    ticket: null,
    productionAuthorization: false,
    independentRecoveryProven: false,
    ...patch,
  };
}
beforeEach(() => {
  requestInvoke.mockReset();
  workerInvoke.mockReset().mockResolvedValue({
    scopeCreated: false,
    busy: false,
    stopRequested: false,
    completedMoves: 0,
    totalMoves: 0,
    job: null,
    error: null,
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = false;
    },
  });
});
function open() {
  act(() => {
    window.dispatchEvent(new Event('stillharbour:rename-lab'));
  });
}
it('requires acknowledgment and an actual grant separately for forward and reverse targets', async () => {
  requestInvoke
    .mockResolvedValueOnce(review())
    .mockResolvedValueOnce(review({ ticket: 'forward-ticket' }))
    .mockResolvedValueOnce(review({ state: 'completed', completedSteps: 10 }))
    .mockResolvedValueOnce(review({ state: 'completed', completedSteps: 10 }))
    .mockResolvedValueOnce(review({ planId: 'reverse', reverse: true }))
    .mockResolvedValueOnce(review({ planId: 'reverse', reverse: true, ticket: 'reverse-ticket' }))
    .mockResolvedValueOnce(
      review({ planId: 'reverse', reverse: true, state: 'completed', completedSteps: 10 }),
    )
    .mockResolvedValueOnce(
      review({ planId: 'reverse', reverse: true, state: 'completed', completedSteps: 10 }),
    );
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Create generated rename fixture' }));
  const approve = await screen.findByRole('button', { name: 'Approve generated targets' });
  expect(approve).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(approve);
  fireEvent.click(await screen.findByRole('button', { name: 'Run reviewed generated rename' }));
  expect(requestInvoke).toHaveBeenCalledWith('test_generated_rename', {
    request: { kind: 'start', plan_id: 'forward', ticket: 'forward-ticket' },
  });
  fireEvent.click(await screen.findByRole('button', { name: 'Review generated undo' }));
  const reverseApprove = await screen.findByRole('button', { name: 'Approve generated targets' });
  expect(reverseApprove).toBeDisabled();
  expect(
    screen.queryByRole('button', { name: 'Run reviewed generated undo' }),
  ).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(reverseApprove);
  fireEvent.click(await screen.findByRole('button', { name: 'Run reviewed generated undo' }));
  await screen.findByText('Undo · completed · 10 / 10 journaled steps');
  expect(requestInvoke).toHaveBeenCalledWith('test_generated_rename', {
    request: { kind: 'start', plan_id: 'reverse', ticket: 'reverse-ticket' },
  });
});
it('disposes a late grant after close and requires fresh inspection on return', async () => {
  let resolve!: (value: GeneratedRenameReview) => void;
  requestInvoke.mockResolvedValueOnce(review()).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Create generated rename fixture' }));
  await screen.findByRole('checkbox');
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve generated targets' }));
  fireEvent.click(screen.getByRole('button', { name: 'Close fixture review' }));
  await act(async () => resolve(review({ ticket: 'late-ticket' })));
  open();
  expect(
    screen.queryByRole('button', { name: 'Run reviewed generated rename' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Approve generated targets' })).toBeDisabled();
  expect(requestInvoke).toHaveBeenCalledTimes(2);
});
it('retains an error and exposes fresh inspection instead of reporting completed work', async () => {
  requestInvoke
    .mockRejectedValueOnce(new Error('Native identity changed'))
    .mockResolvedValueOnce(review({ state: 'needs_continuation', pendingIntent: true }));
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Create generated rename fixture' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Native identity changed');
  fireEvent.click(screen.getByRole('button', { name: 'Inspect retained rename state' }));
  await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('needs_continuation'));
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(
    screen.queryByRole('button', { name: 'Run reviewed generated rename' }),
  ).not.toBeInTheDocument();
});
it('requires fresh approval for a catalog commit after native movement has completed', async () => {
  const pending = review({ state: 'needs_catalog_commit', completedSteps: 10 });
  requestInvoke
    .mockResolvedValueOnce(review())
    .mockResolvedValueOnce(review({ ticket: 'native-ticket' }))
    .mockRejectedValueOnce(new Error('Catalog commit denied'))
    .mockResolvedValueOnce(pending)
    .mockResolvedValueOnce({ ...pending, ticket: 'catalog-ticket' })
    .mockResolvedValueOnce(review({ state: 'completed', completedSteps: 10 }))
    .mockResolvedValueOnce(review({ state: 'completed', completedSteps: 10 }));
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Create generated rename fixture' }));
  await screen.findByRole('checkbox');
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve generated targets' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Run reviewed generated rename' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog commit denied');
  fireEvent.click(screen.getByRole('button', { name: 'Inspect retained rename state' }));
  await screen.findByText(
    'Native movement is complete. Fresh approval checks the files and commits catalog paths without repeating moves.',
  );
  expect(screen.getByRole('button', { name: 'Approve generated targets' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: 'Review generated undo' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve generated targets' }));
  fireEvent.click(
    await screen.findByRole('button', { name: 'Commit reviewed generated catalog paths' }),
  );
  await screen.findByText('Catalog paths committed with stable asset IDs.');
  expect(requestInvoke).toHaveBeenCalledWith('test_generated_rename', {
    request: { kind: 'start', plan_id: 'forward', ticket: 'catalog-ticket' },
  });
});
it('observes an active worker, stops it explicitly and requires fresh inspection before another grant', async () => {
  const job: OriginalOperationJob = {
    id: 'worker-job',
    planId: 'forward',
    sourceRootId: 'root',
    kind: 'rename',
    state: 'running',
    totalMoves: 10,
    verifiedMoves: 0,
    startedAt: 'now',
    updatedAt: 'now',
    message: 'Native worker active.',
    generatedFixtureOnly: true,
  };
  let worker: GeneratedWorkerStatus = {
    scopeCreated: false,
    busy: false,
    stopRequested: false,
    completedMoves: 0,
    totalMoves: 0,
    job: null,
    error: null,
  };
  workerInvoke.mockImplementation(async (_command, args) => {
    if (args.request.kind === 'stop')
      worker = {
        ...worker,
        busy: false,
        stopRequested: true,
        completedMoves: 1,
        job: { ...job, state: 'needs_recovery', message: 'Stopped; inspect retained files.' },
      };
    return worker;
  });
  requestInvoke.mockImplementation(async (_command, args) => {
    if (args.request.kind === 'create') return review();
    if (args.request.kind === 'approve') return review({ ticket: 'worker-ticket' });
    if (args.request.kind === 'start') {
      worker = {
        scopeCreated: true,
        busy: true,
        stopRequested: false,
        completedMoves: 1,
        totalMoves: 10,
        job,
        error: null,
      };
      return review({ jobs: [{ ...job, state: 'queued' }] });
    }
    return review({ state: 'needs_continuation', completedSteps: 1, jobs: [worker.job!] });
  });
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Create generated rename fixture' }));
  await screen.findByRole('checkbox');
  fireEvent.click(screen.getByRole('checkbox'));
  fireEvent.click(screen.getByRole('button', { name: 'Approve generated targets' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Run reviewed generated rename' }));
  const stop = await screen.findByRole('button', { name: 'Stop after current step' });
  expect(screen.getByRole('button', { name: 'Inspect retained rename state' })).toBeDisabled();
  fireEvent.click(stop);
  await screen.findByText('Rename · needs_continuation · 1 / 10 journaled steps');
  expect(workerInvoke).toHaveBeenCalledWith('test_generated_rename_worker', {
    request: { kind: 'stop', job_id: 'worker-job' },
  });
  expect(screen.getByRole('button', { name: 'Approve generated targets' })).toBeDisabled();
  expect(
    screen.queryByRole('button', { name: 'Run reviewed generated rename' }),
  ).not.toBeInTheDocument();
});
it('retires a late worker response when its dialog closes without sending an implicit stop', async () => {
  let finish!: (value: GeneratedWorkerStatus) => void;
  workerInvoke.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  render(<GeneratedRenameLab />);
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Close fixture review' }));
  await act(async () =>
    finish({
      scopeCreated: true,
      busy: true,
      stopRequested: false,
      completedMoves: 1,
      totalMoves: 10,
      job: null,
      error: null,
    }),
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(workerInvoke).toHaveBeenCalledTimes(1);
  expect(workerInvoke).not.toHaveBeenCalledWith(
    'test_generated_rename_worker',
    expect.objectContaining({ request: expect.objectContaining({ kind: 'stop' }) }),
  );
});
