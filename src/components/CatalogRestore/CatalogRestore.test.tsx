import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  CatalogRestoreReview,
  SnapshotJob,
  SnapshotReply,
  SnapshotRequest,
} from '../../bindings';
import { libraryData, pickDirectory } from '../../ipc';
import { CatalogRestore, RetainedCatalogRestore } from '@/components/CatalogRestore';
import { catalogSwitchStore } from '../../backup/catalogSwitchState';

vi.mock('../../backup/catalogSwitchState', async (original) => ({
  ...(await original<typeof import('../../backup/catalogSwitchState')>()),
  catalogSwitchStore: { begin: vi.fn(() => 1), prepared: vi.fn(), unconfirmed: vi.fn() },
}));

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const catalogId = 'catalog-1';
const job: SnapshotJob = {
  id: 'restore-1',
  profileId: 'profile-1',
  action: 'restore',
  state: 'completed_verified',
  phase: 'completed',
  files: 4,
  bytes: '1000',
  snapshotId: '1'.repeat(64),
  message: 'Restored bytes compared',
  startedAt: '2026-11-02T00:00:00Z',
  finishedAt: '2026-11-02T00:01:00Z',
  included: true,
  structuralChecked: true,
  dataReadVerified: true,
  restoredCompared: true,
  sourceRetained: true,
  independentRecoveryVerified: false,
};
function initialReview(): CatalogRestoreReview {
  return {
    id: 'catalog-review',
    sourceKind: 'verified_snapshot_restore',
    retainedTransactionId: null,
    digest: 'a'.repeat(64),
    restoreJobId: job.id,
    snapshotId: job.snapshotId!,
    catalogId,
    schema: 17,
    currentRevision: 'newer-current-revision',
    restoredRevision: 'earlier-restored-revision',
    roots: ['Family', 'Archive'].map((label, index) => ({
      rootId: `root-${index}`,
      label,
      savedDisplayPath: `/generated/saved-${index}`,
      confirmedDisplayPath: null,
      confirmed: false,
    })),
    approvalAvailable: false,
    warning: 'The current catalog and newer edits are preserved separately. No merge is performed.',
  };
}
let current: CatalogRestoreReview;
let handler: (request: SnapshotRequest) => Promise<SnapshotReply>;
const activity = vi.fn();
beforeEach(() => {
  vi.clearAllMocks();
  current = initialReview();
  handler = async (request) => {
    if (
      request.kind === 'catalog_restore_review' ||
      request.kind === 'catalog_restore_review_retained'
    )
      return { kind: 'catalog_restore_review', data: current };
    if (request.kind === 'catalog_restore_map') {
      current = {
        ...current,
        digest: current.digest === 'a'.repeat(64) ? 'b'.repeat(64) : 'c'.repeat(64),
        roots: current.roots.map((root) =>
          root.rootId === request.root_id
            ? { ...root, confirmed: true, confirmedDisplayPath: `/native/confirmed/${root.rootId}` }
            : root,
        ),
      };
      current.approvalAvailable = current.roots.every((root) => root.confirmed);
      return { kind: 'catalog_restore_review', data: current };
    }
    if (request.kind === 'catalog_restore_approve')
      return {
        kind: 'catalog_restore_approval',
        data: { id: 'catalog-approval', reviewId: request.review_id, digest: request.digest },
      };
    if (request.kind === 'catalog_restore_prepare')
      return {
        kind: 'catalog_restore_prepared',
        data: {
          transactionId: 'prepared-transaction',
          catalogId,
          restartRequired: true,
          currentCatalogRetained: true,
          message: 'Verified catalog switch awaits restart.',
        },
      };
    throw new Error('Unexpected catalog fixture request');
  };
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind !== 'backup_snapshots') throw new Error('Unexpected catalog envelope');
    return (await handler(request.request)) as never;
  });
  vi.mocked(pickDirectory).mockResolvedValue({
    id: 'opaque-native-selection',
    displayPath: '/display-only-never-input',
    identityAvailable: true,
  });
});
function open(value = job) {
  return render(
    <CatalogRestore
      job={value}
      catalogId={catalogId}
      disabled={false}
      onActivityChange={activity}
    />,
  );
}
async function review() {
  fireEvent.click(screen.getByRole('button', { name: 'Review catalog switch' }));
  await screen.findByText('newer-current-revision');
}
async function mapRoots() {
  for (const label of ['Family', 'Archive']) {
    fireEvent.click(screen.getByRole('button', { name: `Choose native location for ${label}` }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: `Choose native location for ${label}` }),
      ).toBeEnabled(),
    );
  }
}
async function approved() {
  await review();
  await mapRoots();
  fireEvent.click(
    screen.getByRole('checkbox', { name: /I reviewed these exact catalog revisions/ }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Approve catalog switch' }));
  await screen.findByText('Approval recorded. Catalog preparation has not started.');
}
function kinds() {
  return vi
    .mocked(libraryData)
    .mock.calls.map(([request]) =>
      request.kind === 'backup_snapshots' ? request.request.kind : request.kind,
    );
}

it('requires actual whole-restore evidence and performs no automatic review or switch', () => {
  const view = open({ ...job, restoredCompared: false });
  expect(screen.queryByRole('button', { name: 'Review catalog switch' })).not.toBeInTheDocument();
  expect(screen.getByText(/requires a completed whole-snapshot restore/)).toBeVisible();
  view.rerender(
    <CatalogRestore job={job} catalogId={catalogId} disabled={false} onActivityChange={activity} />,
  );
  expect(screen.getByRole('button', { name: 'Review catalog switch' })).toBeEnabled();
  expect(libraryData).not.toHaveBeenCalled();
});
it('shows current and restored revisions and requires every fresh native mapping', async () => {
  open();
  await review();
  expect(screen.getByText('earlier-restored-revision')).toBeVisible();
  expect(screen.getByText(catalogId)).toBeVisible();
  expect(screen.getByText(/No merge is performed/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await screen.findByText('Native identity confirmed: /native/confirmed/root-0');
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'catalog_restore_map',
        review_id: 'catalog-review',
        root_id: 'root-0',
        selection_id: 'opaque-native-selection',
      },
    },
    'backup_snapshots',
  );
  expect(JSON.stringify(vi.mocked(libraryData).mock.calls)).not.toContain(
    '/display-only-never-input',
  );
});
it('separates exact approval from preparation and reports a pending restart without activation success', async () => {
  open();
  await approved();
  expect(kinds()).not.toContain('catalog_restore_prepare');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'catalog_restore_approve',
        review_id: current.id,
        digest: current.digest,
      },
    },
    'backup_snapshots',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Prepare catalog switch' }));
  await screen.findByText(/Pending restart. The current catalog is retained/);
  expect(screen.getByText(/does not confirm that the restored catalog is active/)).toBeVisible();
  expect(screen.getByText(/Close and reopen Stillharbour to validate and apply/)).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Prepare catalog switch' })).not.toBeInTheDocument();
  expect(activity).toHaveBeenLastCalledWith(job.id, true);
});
it('retires approval and acknowledgment before changing a native root mapping', async () => {
  open();
  await approved();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Choose native location for Family' })).toBeEnabled(),
  );
  expect(screen.queryByRole('button', { name: 'Prepare catalog switch' })).not.toBeInTheDocument();
  expect(
    screen.getByRole('checkbox', { name: /I reviewed these exact catalog revisions/ }),
  ).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
  expect(kinds()).not.toContain('catalog_restore_prepare');
});
it('does not manufacture a mapping after native selection is canceled or rejected', async () => {
  vi.mocked(pickDirectory)
    .mockResolvedValueOnce(null)
    .mockRejectedValueOnce(new Error('Native picker unavailable'));
  open();
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Choose native location for Family' })).toBeEnabled(),
  );
  expect(kinds()).not.toContain('catalog_restore_map');
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('alert')).toHaveTextContent('Native picker unavailable');
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
});
it('preserves backend mapping errors without claiming a confirmed identity', async () => {
  const normal = handler;
  handler = (request) =>
    request.kind === 'catalog_restore_map'
      ? Promise.reject(new Error('Native root identity does not match'))
      : normal(request);
  open();
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await screen.findByRole('alert');
  expect(screen.queryByText(/Native identity confirmed:/)).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
});
it('rejects a mapping response that silently omits another required root', async () => {
  const normal = handler;
  handler = async (request) => {
    const value = await normal(request);
    if (request.kind === 'catalog_restore_map' && value.kind === 'catalog_restore_review')
      return {
        ...value,
        data: { ...value.data, roots: value.data.roots.slice(0, 1), approvalAvailable: true },
      };
    return value;
  };
  open();
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('alert')).toHaveTextContent('scope changed');
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
});
it.each(['currentRevision', 'restoredRevision'] as const)(
  'requires a fresh review if %s changes during mapping',
  async (field) => {
    const normal = handler;
    handler = async (request) => {
      const value = await normal(request);
      if (request.kind === 'catalog_restore_map' && value.kind === 'catalog_restore_review')
        return { ...value, data: { ...value.data, [field]: 'changed-revision' } };
      return value;
    };
    open();
    await review();
    fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
    await screen.findByRole('alert');
    expect(screen.getByRole('alert')).toHaveTextContent('scope changed');
    expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
  },
);
it.each(['restoreJobId', 'snapshotId', 'catalogId'] as const)(
  'rejects a review for a different %s',
  async (field) => {
    current = { ...current, [field]: 'another-native-identity' };
    open();
    fireEvent.click(screen.getByRole('button', { name: 'Review catalog switch' }));
    await screen.findByRole('alert');
    expect(
      screen.queryByRole('button', { name: 'Approve catalog switch' }),
    ).not.toBeInTheDocument();
  },
);
it('surfaces unsupported or failed catalog review without an empty success panel', async () => {
  handler = () => Promise.reject(new Error('Restored catalog platform or schema is unsupported'));
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Review catalog switch' }));
  await screen.findByRole('alert');
  expect(screen.getByRole('alert')).toHaveTextContent('unsupported');
  expect(screen.queryByText('newer-current-revision')).not.toBeInTheDocument();
});
it('rejects mismatched approval and never exposes preparation', async () => {
  const normal = handler;
  handler = async (request) =>
    request.kind === 'catalog_restore_approve'
      ? {
          kind: 'catalog_restore_approval',
          data: { id: 'wrong', reviewId: 'other', digest: 'd'.repeat(64) },
        }
      : normal(request);
  open();
  await review();
  await mapRoots();
  fireEvent.click(
    screen.getByRole('checkbox', { name: /I reviewed these exact catalog revisions/ }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Approve catalog switch' }));
  await screen.findByRole('alert');
  expect(screen.queryByRole('button', { name: 'Prepare catalog switch' })).not.toBeInTheDocument();
});
it('consumes preparation approval on a failed or lost response and requires reopening', async () => {
  const normal = handler;
  handler = (request) =>
    request.kind === 'catalog_restore_prepare'
      ? Promise.reject(new Error('Native preparation response was lost'))
      : normal(request);
  open();
  await approved();
  fireEvent.click(screen.getByRole('button', { name: 'Prepare catalog switch' }));
  await screen.findByRole('alert');
  expect(screen.getByText(/Close and reopen Stillharbour to reconcile/)).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Prepare catalog switch' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Review catalog switch' })).not.toBeInTheDocument();
  expect(activity).toHaveBeenLastCalledWith(job.id, true);
});
it('does not claim a prepared switch when retention evidence is absent', async () => {
  const normal = handler;
  handler = async (request) =>
    request.kind === 'catalog_restore_prepare'
      ? {
          kind: 'catalog_restore_prepared',
          data: {
            transactionId: 'bad',
            catalogId,
            restartRequired: true,
            currentCatalogRetained: false,
            message: 'Unproved',
          },
        }
      : normal(request);
  open();
  await approved();
  fireEvent.click(screen.getByRole('button', { name: 'Prepare catalog switch' }));
  await screen.findByRole('alert');
  expect(screen.queryByText('Unproved')).not.toBeInTheDocument();
  expect(
    screen.queryByText(/Pending restart. The current catalog is retained/),
  ).not.toBeInTheDocument();
});
it('retires an outstanding review when the selected restore job changes', async () => {
  let finish!: (reply: SnapshotReply) => void;
  handler = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  const view = open();
  fireEvent.click(screen.getByRole('button', { name: 'Review catalog switch' }));
  view.rerender(
    <CatalogRestore
      job={{ ...job, id: 'restore-2' }}
      catalogId={catalogId}
      disabled={false}
      onActivityChange={activity}
    />,
  );
  await act(async () => {
    finish({ kind: 'catalog_restore_review', data: current });
  });
  expect(screen.queryByText('newer-current-revision')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Review catalog switch' })).toBeEnabled();
});
it('never submits an old picker selection after a restore scope change', async () => {
  let finish!: (selection: Awaited<ReturnType<typeof pickDirectory>>) => void;
  vi.mocked(pickDirectory).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve;
      }),
  );
  const view = open();
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Choose native location for Family' }));
  view.rerender(
    <CatalogRestore
      job={{ ...job, id: 'restore-2' }}
      catalogId={catalogId}
      disabled={false}
      onActivityChange={activity}
    />,
  );
  await act(async () => {
    finish({ id: 'retired-selection', displayPath: '/display-only', identityAvailable: true });
  });
  expect(kinds()).not.toContain('catalog_restore_map');
});

it.each(['prepared', 'lost'] as const)(
  'settles global %s state after the initiating panel unmounts',
  async (outcome) => {
    const normal = handler;
    let finish!: (reply: SnapshotReply) => void;
    let fail!: (error: Error) => void;
    handler = (request) =>
      request.kind === 'catalog_restore_prepare'
        ? new Promise((resolve, reject) => {
            finish = resolve;
            fail = reject;
          })
        : normal(request);
    const view = open();
    await approved();
    fireEvent.click(screen.getByRole('button', { name: 'Prepare catalog switch' }));
    expect(catalogSwitchStore.begin).toHaveBeenCalledWith({
      catalogId,
      sourceKind: current.sourceKind,
      restoreJobId: job.id,
      snapshotId: job.snapshotId,
      retainedTransactionId: null,
      reviewDigest: current.digest,
    });
    view.unmount();
    await act(async () => {
      if (outcome === 'prepared')
        finish({
          kind: 'catalog_restore_prepared',
          data: {
            transactionId: 'transaction',
            catalogId,
            restartRequired: true,
            currentCatalogRetained: true,
            message: 'Native prepared',
          },
        });
      else fail(new Error('Native response lost after unmount'));
    });
    if (outcome === 'prepared')
      expect(catalogSwitchStore.prepared).toHaveBeenCalledWith(
        1,
        expect.objectContaining({ transactionId: 'transaction' }),
      );
    else
      expect(catalogSwitchStore.unconfirmed).toHaveBeenCalledWith(
        1,
        'Native response lost after unmount',
      );
  },
);

function retainedReview(): CatalogRestoreReview {
  return {
    ...initialReview(),
    sourceKind: 'retained_pre_restore',
    restoreJobId: null,
    snapshotId: null,
    retainedTransactionId: 'latest-native-transaction',
  };
}
function openRetained(transactionId = 'latest-native-transaction') {
  return render(
    <RetainedCatalogRestore
      transactionId={transactionId}
      catalogId={catalogId}
      disabled={false}
      onActivityChange={activity}
    />,
  );
}
async function loadRetained() {
  fireEvent.click(screen.getByRole('button', { name: 'Review latest retained catalog' }));
  await screen.findByText('newer-current-revision');
}
it('reviews only the native latest retained source without inventing snapshot or job identity', async () => {
  current = retainedReview();
  openRetained();
  expect(libraryData).not.toHaveBeenCalled();
  await loadRetained();
  expect(screen.getByText(/Source: Local catalog saved before switch/)).toBeVisible();
  expect(screen.queryByText(job.snapshotId!)).not.toBeInTheDocument();
  expect(screen.queryByText(job.id)).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Approve catalog switch' })).toBeDisabled();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'catalog_restore_review_retained',
        transaction_id: 'latest-native-transaction',
      },
    },
    'backup_snapshots',
  );
  expect(kinds()).not.toContain('catalog_restore_review');
});
it('creates a new reviewed and approved retained-source preparation that preserves current edits again', async () => {
  current = retainedReview();
  openRetained();
  await loadRetained();
  await mapRoots();
  expect(screen.getByText(/preserved again in a new transaction/)).toBeVisible();
  fireEvent.click(
    screen.getByRole('checkbox', { name: /I reviewed these exact catalog revisions/ }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Approve catalog switch' }));
  await screen.findByRole('button', { name: 'Prepare catalog switch' });
  expect(kinds()).not.toContain('catalog_restore_prepare');
  fireEvent.click(screen.getByRole('button', { name: 'Prepare catalog switch' }));
  await screen.findByText(/Pending restart. The current catalog is retained/);
  expect(catalogSwitchStore.begin).toHaveBeenCalledWith({
    catalogId,
    sourceKind: 'retained_pre_restore',
    restoreJobId: null,
    snapshotId: null,
    retainedTransactionId: 'latest-native-transaction',
    reviewDigest: current.digest,
  });
});
it.each(['sourceKind', 'restoreJobId', 'snapshotId', 'retainedTransactionId'] as const)(
  'rejects retained review with mismatched %s attribution',
  async (field) => {
    current = {
      ...retainedReview(),
      [field]: field === 'sourceKind' ? 'verified_snapshot_restore' : 'wrong-source-identity',
    } as CatalogRestoreReview;
    openRetained();
    fireEvent.click(screen.getByRole('button', { name: 'Review latest retained catalog' }));
    await screen.findByRole('alert');
    expect(
      screen.queryByRole('button', { name: 'Approve catalog switch' }),
    ).not.toBeInTheDocument();
  },
);
it('retires an old retained-source response when the latest native transaction changes', async () => {
  let finish!: (reply: SnapshotReply) => void;
  current = retainedReview();
  handler = () =>
    new Promise((resolve) => {
      finish = resolve;
    });
  const view = openRetained();
  fireEvent.click(screen.getByRole('button', { name: 'Review latest retained catalog' }));
  view.rerender(
    <RetainedCatalogRestore
      transactionId="newer-native-transaction"
      catalogId={catalogId}
      disabled={false}
      onActivityChange={activity}
    />,
  );
  await act(async () => {
    finish({ kind: 'catalog_restore_review', data: current });
  });
  expect(screen.queryByText('newer-current-revision')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Review latest retained catalog' })).toBeEnabled();
});
