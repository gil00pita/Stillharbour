import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { SnapshotProfile, SnapshotReview, SnapshotJob, SyncReply } from '../bindings';
import { libraryData } from '../ipc';
import { OldDestinationBackup } from './OldDestinationBackup';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const profile: SnapshotProfile = {
  id: 'repository-profile',
  label: 'Generated recovery repository',
  catalogId: 'catalog',
  rootIds: ['source'],
  protocol: 'local',
  destinationDisplay: '/repository',
  repositoryId: 'restic-repo',
  createdAt: '2026-11-02',
  hostFingerprint: null,
  insecureTransport: false,
  credentialsPresent: true,
  status: 'ready',
};
const review: SnapshotReview = {
  id: 'old-review',
  profileId: profile.id,
  action: 'backup',
  digest: 'a'.repeat(64),
  files: 2,
  bytes: '8',
  rootIds: ['old-dataset'],
  sourceDisplayPaths: ['/destination/existing-dataset'],
  items: [
    { displayPath: 'Old destination/edited.jpg', bytes: '4', fullDigest: 'b'.repeat(64) },
    { displayPath: 'Old destination/edited.xmp', bytes: '4', fullDigest: 'c'.repeat(64) },
  ],
  catalogIncluded: true,
  sourceRetained: true,
  targetDisplay: '/repository/old-versions',
  issues: ['Only exact old destination files and companions.'],
  executionAvailable: true,
  unavailableReason: null,
};
const job: SnapshotJob = {
  id: 'old-job',
  profileId: profile.id,
  action: 'backup',
  state: 'completed_verified',
  phase: 'completed',
  files: 2,
  bytes: '8',
  snapshotId: 'd'.repeat(64),
  message: 'Exact old destination versions recovered.',
  startedAt: '2026-11-02',
  finishedAt: '2026-11-02',
  included: true,
  structuralChecked: true,
  dataReadVerified: true,
  restoredCompared: true,
  sourceRetained: true,
  independentRecoveryVerified: false,
};
beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (input) => {
      if (input.kind === 'backup_sync') {
        if (input.request.kind === 'review_replacement')
          throw new Error('Essential metadata and independent recovery remain unverified.');
        return { kind: 'recovery_backup', data: review };
      }
      if (input.kind === 'backup_snapshots') {
        if (input.request.kind === 'profiles') return { kind: 'profiles', data: [profile] };
        if (input.request.kind === 'backup_approve')
          return {
            kind: 'approval',
            data: { id: 'approval', reviewId: review.id, digest: review.digest },
          };
        if (input.request.kind === 'backup_start') return { kind: 'job', data: job };
      }
      throw new Error('Unexpected request');
    });
});
function open() {
  return render(
    <OldDestinationBackup
      key="entry"
      runId="run"
      entryId="edited-entry"
      disabled={false}
      onActivityChange={vi.fn()}
      onReplacementReview={vi.fn()}
    />,
  );
}
async function select() {
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Recovery snapshot profile' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByRole('combobox', { name: 'Recovery snapshot profile' }), {
    target: { value: profile.id },
  });
}
it('reviews the actual old files and companions and separately approves their snapshot without enabling replacement', async () => {
  open();
  await select();
  fireEvent.click(screen.getByRole('button', { name: 'Review old destination recovery backup' }));
  const panel = await screen.findByRole('region', { name: 'Snapshot action review' });
  expect(within(panel).getByText('/destination/existing-dataset')).toBeVisible();
  expect(within(panel).getByText('Old destination/edited.xmp')).toBeVisible();
  expect(within(panel).getByText('c'.repeat(64))).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_sync',
      request: {
        kind: 'review_recovery_backup',
        run_id: 'run',
        entry_id: 'edited-entry',
        profile_id: profile.id,
      },
    },
    'backup_sync',
  );
  expect(screen.queryByRole('button', { name: 'Start encrypted backup' })).not.toBeInTheDocument();
  fireEvent.click(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ }));
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this snapshot action' }));
  const start = await within(panel).findByRole('button', { name: 'Start encrypted backup' });
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'backup_start',
      ),
  ).toBe(false);
  fireEvent.click(start);
  expect(await screen.findByText('d'.repeat(64))).toBeVisible();
  expect(screen.getByText(/does not automatically enable replacement/)).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: { kind: 'backup_start', review_id: review.id, approval_id: 'approval' },
    },
    'backup_snapshots',
  );
  fireEvent.click(
    screen.getByRole('button', { name: 'Inspect retained replacement gate for this snapshot' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Essential metadata and independent recovery remain unverified.',
  );
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_sync',
      request: {
        kind: 'review_replacement',
        run_id: 'run',
        entry_id: 'edited-entry',
        recovery_evidence_id: 'd'.repeat(64),
      },
    },
    'backup_sync',
  );
  expect(screen.queryByRole('button', { name: /^Start replace/ })).not.toBeInTheDocument();
});
it('discards an old-destination review that arrives after choosing another comparison entry', async () => {
  let finish!: (value: SyncReply) => void;
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (input, kind) =>
    input.kind === 'backup_sync'
      ? new Promise<SyncReply>((resolve) => {
          finish = resolve;
        })
      : fallback(input, kind),
  );
  const view = open();
  await select();
  fireEvent.click(screen.getByRole('button', { name: 'Review old destination recovery backup' }));
  await waitFor(() => expect(finish).toBeDefined());
  view.rerender(
    <OldDestinationBackup
      key="other"
      runId="run"
      entryId="other-entry"
      disabled={false}
      onActivityChange={vi.fn()}
      onReplacementReview={vi.fn()}
    />,
  );
  finish({ kind: 'recovery_backup', data: review });
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Recovery snapshot profile' })).toBeEnabled(),
  );
  expect(screen.queryByRole('region', { name: 'Snapshot action review' })).not.toBeInTheDocument();
});
