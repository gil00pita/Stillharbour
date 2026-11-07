import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '../../../test/render';
import { library } from '../../../ipc';
import { StructureProtection } from './StructureProtection';

vi.mock('../../../ipc', async (original) => ({
  ...(await original<typeof import('../../../ipc')>()),
  library: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(library)
    .mockReset()
    .mockResolvedValue({
      kind: 'organization_protection_status',
      data: { blockers: [] },
    });
});
afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
});

it('checks on entry in a hidden window, pauses background polls and refreshes on focus', async () => {
  vi.useFakeTimers();
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  render(<StructureProtection />);
  await act(async () => {
    await Promise.resolve();
  });
  expect(library).toHaveBeenCalledTimes(1);
  expect(screen.getByText(/No active backup or sync/)).toBeVisible();
  await act(async () => vi.advanceTimersByTimeAsync(6000));
  expect(library).toHaveBeenCalledTimes(1);
  await act(async () => fireEvent(window, new Event('focus')));
  expect(library).toHaveBeenCalledTimes(2);
});

it('checks native protection state on entry and shows blockers with recovery actions', async () => {
  vi.mocked(library).mockResolvedValue({
    kind: 'organization_protection_status',
    data: {
      blockers: [
        'backup_active',
        'sync_active',
        'backup_recovery_required',
        'sync_recovery_required',
        'backup_scheduled',
      ],
    },
  });
  render(<StructureProtection />);
  expect(screen.getByText('Checking backup, sync and schedules…')).toBeVisible();
  expect(await screen.findByText(/Turn off enabled backup schedules/)).toBeVisible();
  expect(screen.getByText(/Wait for the active backup/)).toBeVisible();
  expect(screen.getByText(/Wait for the active sync/)).toBeVisible();
  expect(screen.getByText(/Review unfinished backup jobs/)).toBeVisible();
  expect(screen.getByText(/resolve paused and interrupted jobs/)).toBeVisible();
  expect(library).toHaveBeenCalledExactlyOnceWith({ kind: 'organization_protection_status' });
});
it('keeps failures blocked, and rechecks without canceling jobs or changing schedules', async () => {
  vi.mocked(library).mockRejectedValueOnce(new Error('offline'));
  render(<StructureProtection />);
  expect(await screen.findByText(/Could not verify backup and sync/)).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Check again' }));
  expect(await screen.findByText(/No active backup or sync/)).toBeVisible();
  expect(vi.mocked(library).mock.calls.map(([request]) => request.kind)).toEqual([
    'organization_protection_status',
    'organization_protection_status',
  ]);
});
it('refreshes changes in native jobs and schedules, coalesces pending requests and stops after exit', async () => {
  vi.useFakeTimers();
  let resolve!: (reply: Awaited<ReturnType<typeof library>>) => void;
  vi.mocked(library).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const view = render(<StructureProtection />);
  await act(async () => vi.advanceTimersByTimeAsync(6000));
  expect(library).toHaveBeenCalledTimes(1);
  await act(async () =>
    resolve({ kind: 'organization_protection_status', data: { blockers: [] } }),
  );
  expect(screen.getByText(/No active backup or sync/)).toBeVisible();
  vi.mocked(library).mockResolvedValue({
    kind: 'organization_protection_status',
    data: { blockers: ['backup_scheduled'] },
  });
  await act(async () => vi.advanceTimersByTimeAsync(2000));
  expect(screen.getByText(/Turn off enabled backup schedules/)).toBeVisible();
  const count = vi.mocked(library).mock.calls.length;
  view.unmount();
  await act(async () => vi.advanceTimersByTimeAsync(4000));
  expect(library).toHaveBeenCalledTimes(count);
});
it('rechecks when the application gains focus and rejects an unexpected native reply', async () => {
  render(<StructureProtection />);
  await screen.findByText(/No active backup or sync/);
  vi.mocked(library).mockResolvedValue({ kind: 'updated', data: 0 });
  fireEvent(window, new Event('focus'));
  expect(await screen.findByText(/Could not verify backup and sync/)).toBeVisible();
});
