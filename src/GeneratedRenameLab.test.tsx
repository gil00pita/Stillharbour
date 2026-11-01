import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { GeneratedRenameReview } from './bindings';
import { GeneratedRenameLab } from './GeneratedRenameLab';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
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
  vi.mocked(invoke).mockReset();
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
  vi.mocked(invoke)
    .mockResolvedValueOnce(review())
    .mockResolvedValueOnce(review({ ticket: 'forward-ticket' }))
    .mockResolvedValueOnce(review({ state: 'completed', completedSteps: 10 }))
    .mockResolvedValueOnce(review({ planId: 'reverse', reverse: true }))
    .mockResolvedValueOnce(review({ planId: 'reverse', reverse: true, ticket: 'reverse-ticket' }))
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
  expect(invoke).toHaveBeenCalledWith('test_generated_rename', {
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
  expect(invoke).toHaveBeenLastCalledWith('test_generated_rename', {
    request: { kind: 'start', plan_id: 'reverse', ticket: 'reverse-ticket' },
  });
});
it('disposes a late grant after close and requires fresh inspection on return', async () => {
  let resolve!: (value: GeneratedRenameReview) => void;
  vi.mocked(invoke)
    .mockResolvedValueOnce(review())
    .mockImplementationOnce(
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
  expect(invoke).toHaveBeenCalledTimes(2);
});
it('retains an error and exposes fresh inspection instead of reporting completed work', async () => {
  vi.mocked(invoke)
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
  vi.mocked(invoke)
    .mockResolvedValueOnce(review())
    .mockResolvedValueOnce(review({ ticket: 'native-ticket' }))
    .mockRejectedValueOnce(new Error('Catalog commit denied'))
    .mockResolvedValueOnce(pending)
    .mockResolvedValueOnce({ ...pending, ticket: 'catalog-ticket' })
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
  expect(invoke).toHaveBeenLastCalledWith('test_generated_rename', {
    request: { kind: 'start', plan_id: 'forward', ticket: 'catalog-ticket' },
  });
});
