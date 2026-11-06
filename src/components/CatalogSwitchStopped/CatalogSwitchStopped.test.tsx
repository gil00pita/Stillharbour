import { act, render, screen } from '../../test/render';
import { expect, it, vi } from 'vitest';
import App from '@/components/App';
import { BackupScreen } from '@/components/BackupScreen';
import { CatalogSwitchStopped } from '@/components/CatalogSwitchStopped';
import { catalogSwitchStore, createCatalogSwitchStore } from '../../backup/catalogSwitchState';
import type { CatalogRestorePrepared, LibraryStatus } from '../../bindings';
import { getPreferences, getStatus, libraryData } from '../../ipc';
import { fixturePreferences } from '../../library/__tests__/fixtures';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  getStatus: vi.fn(),
  getPreferences: vi.fn(),
  libraryData: vi.fn(),
}));
const context = {
  sourceKind: 'verified_snapshot_restore' as const,
  snapshotId: '1'.repeat(64),
  retainedTransactionId: null,
  catalogId: 'catalog-1',
  restoreJobId: 'restore-1',
  reviewDigest: 'a'.repeat(64),
};
const prepared: CatalogRestorePrepared = {
  transactionId: 'transaction-1',
  catalogId: context.catalogId,
  restartRequired: true,
  currentCatalogRetained: true,
  message: 'Native switch prepared.',
};

it('keeps preparing, prepared and unconfirmed states distinct and rejects unproved success', () => {
  const store = createCatalogSwitchStore();
  const changed = vi.fn();
  const unsubscribe = store.subscribe(changed);
  const ticket = store.begin(context);
  expect(store.getSnapshot()?.phase).toBe('preparing');
  expect(() => store.prepared(ticket, { ...prepared, currentCatalogRetained: false })).toThrow(
    /preservation/,
  );
  expect(store.getSnapshot()?.phase).toBe('preparing');
  store.unconfirmed(ticket, 'Native response lost');
  store.prepared(ticket, prepared);
  expect(store.getSnapshot()).toMatchObject({
    phase: 'unconfirmed',
    message: 'Native response lost',
  });
  expect(() => store.begin(context)).toThrow(/closing and reopening/);
  expect(changed).toHaveBeenCalledTimes(2);
  unsubscribe();
});
it('ignores a completion from another preparation ticket', () => {
  const store = createCatalogSwitchStore();
  const ticket = store.begin(context);
  store.prepared(ticket + 1, prepared);
  expect(store.getSnapshot()?.phase).toBe('preparing');
  store.prepared(ticket, prepared);
  expect(store.getSnapshot()).toMatchObject({ phase: 'prepared', prepared });
});
it('keeps retained source identity explicit and rejects invented snapshot or job attribution', () => {
  const retained = {
    ...context,
    sourceKind: 'retained_pre_restore' as const,
    restoreJobId: null,
    snapshotId: null,
    retainedTransactionId: 'latest-native-transaction',
  };
  for (const invalid of [
    { ...retained, restoreJobId: 'invented-job' },
    { ...retained, snapshotId: 'invented-snapshot' },
    { ...retained, retainedTransactionId: null },
  ]) {
    expect(() => createCatalogSwitchStore().begin(invalid)).toThrow(/source attribution/);
  }
  const store = createCatalogSwitchStore();
  const ticket = store.begin(retained);
  store.prepared(ticket, prepared);
  expect(store.getSnapshot()).toMatchObject({
    phase: 'prepared',
    sourceKind: 'retained_pre_restore',
    restoreJobId: null,
    snapshotId: null,
    retainedTransactionId: 'latest-native-transaction',
  });
});
it('renders an uncertain stopped session without an activation badge or retry controls', () => {
  render(
    <CatalogSwitchStopped
      state={{
        ...context,
        ticket: 1,
        phase: 'unconfirmed',
        message: 'Preparation response was lost',
      }}
    />,
  );
  expect(screen.getByRole('alert')).toHaveTextContent('Preparation response was lost');
  expect(screen.getByText(/Close and reopen Stillharbour/)).toBeVisible();
  expect(screen.queryAllByRole('button')).toHaveLength(0);
  expect(screen.queryByText('Native switch prepared.')).not.toBeInTheDocument();
});
it('unmounts normal app and backup requests before preparation and retains instructions through late Busy errors', async () => {
  vi.useFakeTimers();
  try {
    const status: LibraryStatus = {
      roots: [],
      jobs: [],
      revision: '0',
      previewGeneration: '0',
      metadataAvailable: true,
      previewAvailable: true,
    };
    vi.mocked(getPreferences).mockResolvedValue(fixturePreferences());
    vi.mocked(getStatus).mockResolvedValue({
      name: 'Stillharbour',
      version: '0.1.0',
      milestone: 'M4',
      sqliteVersion: '3.54.1',
      catalogId: context.catalogId,
      catalogSchema: 17,
      originalMutationsEnabled: false,
    });
    let rejectLate!: (error: Error) => void;
    let statuses = 0;
    vi.mocked(libraryData).mockImplementation(async (request) => {
      if (request.kind === 'status') {
        if (++statuses === 1) return status as never;
        return (await new Promise((_, reject) => {
          rejectLate = reject;
        })) as never;
      }
      if (request.kind === 'backup_profiles') return [] as never;
      throw new Error('Unexpected normal request');
    });
    render(<App />);
    render(<BackupScreen status={status} navigationControl={null} jobsControl={null} />);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByRole('button', { name: 'Choose a folder' })).toBeEnabled();
    expect(screen.getByRole('button', { name: 'Versioned backup' })).toBeEnabled();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1500);
    });
    expect(statuses).toBe(2);
    let ticket = 0;
    await act(async () => {
      ticket = catalogSwitchStore.begin(context);
    });
    expect(screen.queryByRole('button', { name: 'Choose a folder' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Versioned backup' })).not.toBeInTheDocument();
    expect(screen.getAllByRole('heading', { name: 'Preparing the catalog switch' })).toHaveLength(
      2,
    );
    const requests = vi.mocked(libraryData).mock.calls.length;
    await act(async () => {
      rejectLate(new Error('BUSY: normal RPC is stopped'));
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(vi.mocked(libraryData).mock.calls).toHaveLength(requests);
    expect(screen.queryByText(/BUSY: normal RPC/)).not.toBeInTheDocument();
    await act(async () => {
      catalogSwitchStore.prepared(ticket, prepared);
    });
    expect(screen.getAllByRole('heading', { name: 'Catalog switch awaits restart' })).toHaveLength(
      2,
    );
    expect(screen.getAllByText(/restored catalog is pending startup validation/)).toHaveLength(2);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10000);
    });
    expect(vi.mocked(libraryData).mock.calls).toHaveLength(requests);
  } finally {
    vi.useRealTimers();
  }
});
