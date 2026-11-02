import { fireEvent, render, screen, waitFor, within } from '../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { BackupScreen } from './BackupScreen';
import { libraryData, pickDirectory } from '../ipc';
import type {
  ComparisonRun,
  LibraryStatus,
  SyncProfile,
  SyncRequest,
  SyncReply,
  SyncJob,
} from '../bindings';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const status = {
  roots: [
    {
      id: 'base',
      label: 'Family',
      displayPath: '/generated/source',
      isBase: true,
      availability: 'online',
    },
  ],
  jobs: [],
} as unknown as LibraryStatus;
function report(state: ComparisonRun['state'] = 'needs_review'): ComparisonRun {
  return {
    id: 'run',
    policyVersion: 3,
    baselineId: null,
    reviewPlan: null,
    historyMessage: 'No retained observation.',
    catalogId: 'catalog',
    sourceRootId: 'base',
    sourceLabel: 'Family',
    sourceDisplayPath: '/generated/source',
    destinationDisplayPath: '/generated/dest',
    datasetDisplayPath: '/generated/dest/.stillharbour-copy/catalog/base',
    mode: 'verified',
    state,
    startedAt: '2026-11-01T00:00:00Z',
    bytesRead: '16',
    entries: 1,
    issues: [],
    message: 'Read-only comparison finished.',
    planDigest: 'review-only',
    capabilities: {
      readOnlyComparison: true,
      exclusiveCreate: false,
      noReplacePublication: false,
      retainedReplacement: false,
      metadataFidelityVerified: false,
      independentRecoveryVerified: false,
      executionAvailable: false,
    },
  };
}
function readOnlySync(request: SyncRequest): SyncReply {
  if (request.kind === 'capabilities')
    return {
      kind: 'capabilities',
      data: {
        executionAvailable: false,
        retainedReplacementAvailable: false,
        reason: 'Native transfer gates are pending.',
        ioScope: 'Shared application-owned I/O',
        bytesPerSecond: '10485760',
      },
    };
  if (request.kind === 'list') return { kind: 'jobs', data: [] };
  throw new Error('Unexpected safe-copy request in comparison fixture.');
}
beforeEach(() => {
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'selection',
    displayPath: '/generated/dest',
    identityAvailable: true,
  });
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'backup_snapshots' && request.request.kind === 'profiles')
        return { kind: 'profiles', data: [] };
      if (request.kind === 'backup_sync') return readOnlySync(request.request);
      if (request.kind === 'backup_profiles') return [];
      if (request.kind === 'backup_comparison_latest') return null;
      if (request.kind === 'backup_comparison_page')
        return {
          runId: 'run',
          entries: [
            {
              id: 'file',
              companions: ['photo.raw'],
              sourceKind: 'file',
              destinationKind: 'file',
              displayPath: '<script>hostile</script>.xmp',
              sourceBytes: '4',
              destinationBytes: '4',
              sourceModified: null,
              destinationModified: null,
              category: 'different_content',
              proposal: 'review_difference',
              evidence: 'Content differs; no winner selected.',
            },
          ],
          nextOffset: null,
        };
      return report();
    });
});
function open(value = status) {
  return render(<BackupScreen status={value} navigationControl={null} jobsControl={null} />);
}
async function compare() {
  fireEvent.click(screen.getByRole('button', { name: 'Choose destination' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Compare' })).toBeEnabled());
  fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
}

function savedProfile(): SyncProfile {
  return {
    id: 'saved-comparison',
    version: 1,
    catalogId: 'catalog',
    label: 'Pinned destination',
    sourceRootId: 'base',
    sourceLabel: 'Family',
    sourceDisplayPath: '/generated/source',
    destinationDisplayPath: '/generated/dest',
    createdAt: '2026-11-01T00:00:00Z',
  };
}

it('saves only the native destination selection and requires a fresh profile comparison', async () => {
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) =>
    request.kind === 'backup_profile_save' ? savedProfile() : fallback(request, kind),
  );
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Choose destination' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Compare' })).toBeEnabled());
  fireEvent.change(screen.getByRole('textbox', { name: 'Comparison profile name' }), {
    target: { value: 'Pinned destination' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save comparison profile' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_profile_save', selection_id: 'selection', label: 'Pinned destination' },
      'backup_profile',
    ),
  );
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Saved comparison' })).toHaveValue(
      'saved-comparison',
    ),
  );
  expect(screen.getByRole('button', { name: 'Choose destination' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Review append plan' })).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(([request]) => request.kind === 'backup_profile_compare'),
  ).toBe(false);
});

it('keeps a reopened profile on its bound root when the base changes and blocks a missing root', async () => {
  const profile = savedProfile();
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) => {
    if (request.kind === 'backup_profiles') return [profile];
    if (request.kind === 'backup_profile_compare') return { ...report(), profileId: profile.id };
    return fallback(request, kind);
  });
  const view = open();
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Saved comparison' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByRole('combobox', { name: 'Saved comparison' }), {
    target: { value: profile.id },
  });
  const other = { ...status.roots[0]!, id: 'new-base', label: 'New base', isBase: true };
  view.rerender(
    <BackupScreen
      status={{ ...status, roots: [{ ...status.roots[0]!, isBase: false }, other] }}
      navigationControl={null}
      jobsControl={null}
    />,
  );
  expect(screen.getByText('Family')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_profile_compare', profile_id: profile.id, mode: 'verified' },
      'backup_comparison',
    ),
  );
  expect(pickDirectory).not.toHaveBeenCalled();
  view.rerender(
    <BackupScreen
      status={{ ...status, roots: [other] }}
      navigationControl={null}
      jobsControl={null}
    />,
  );
  expect(screen.getByRole('button', { name: 'Compare' })).toBeDisabled();
  expect(screen.getByText(/not an empty library/)).toBeVisible();
});

it('binds the source and sends only the native selection ID and verified policy', async () => {
  open();
  expect(screen.getByText('Family')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Review append plan' })).toBeDisabled();
  expect(screen.queryByRole('button', { name: /choose source/i })).not.toBeInTheDocument();
  await compare();
  await screen.findByText('different content');
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'backup_compare_start', selection_id: 'selection', mode: 'verified' },
    'backup_comparison',
  );
  expect(screen.getByText('<script>hostile</script>.xmp')).toBeVisible();
  expect(document.querySelector('.backup-workspace script')).toBeNull();
  expect(screen.queryByRole('button', { name: /^Start / })).not.toBeInTheDocument();
});
it('retires the previous review after changing comparison policy', async () => {
  open();
  await compare();
  await screen.findByText('different content');
  fireEvent.change(screen.getByRole('combobox', { name: 'Comparison' }), {
    target: { value: 'quick' },
  });
  expect(screen.queryByText('different content')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Compare' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_compare_start', selection_id: 'selection', mode: 'quick' },
      'backup_comparison',
    ),
  );
});
it('inspects escaped observations and clears details when filtering the entire report', async () => {
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) => {
    if (request.kind === 'backup_comparison_page' && request.category === 'blocked')
      return { runId: 'run', entries: [], total: 0, nextOffset: null };
    return fallback(request, kind);
  });
  open();
  await compare();
  fireEvent.click(await screen.findByRole('button', { name: '<script>hostile</script>.xmp' }));
  const details = screen.getByRole('region', { name: 'Entry details' });
  expect(within(details).getByText('<script>hostile</script>.xmp')).toBeVisible();
  expect(within(details).getByText(/Content differs/)).toBeVisible();
  expect(document.querySelector('.backup-workspace script')).toBeNull();
  fireEvent.change(screen.getByRole('combobox', { name: 'Finding filter' }), {
    target: { value: 'blocked' },
  });
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_comparison_page', run_id: 'run', offset: 0, category: 'blocked' },
      'backup_comparison_page',
    ),
  );
  expect(await screen.findByText('Entries 0–0 of 0')).toBeVisible();
  expect(screen.queryByRole('region', { name: 'Entry details' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: /^Start / })).not.toBeInTheDocument();
});
it('shows offline scope and cannot mistake it for an empty library', () => {
  open({ ...status, roots: [{ ...status.roots[0]!, availability: 'offline' }] });
  expect(screen.getByText(/not an empty library/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Compare' })).toBeDisabled();
});
it('surfaces picker and comparison failures without reporting success', async () => {
  vi.mocked(pickDirectory).mockRejectedValueOnce(new Error('Folder is unavailable.'));
  open();
  fireEvent.click(screen.getByRole('button', { name: 'Choose destination' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Folder is unavailable.');
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Comparison could not start.'));
  await compare();
  expect(await screen.findByRole('alert')).toHaveTextContent('Comparison could not start.');
  expect(screen.queryByText('Read-only comparison finished.')).not.toBeInTheDocument();
});
it('reopens and cancels the active backend run without starting another job', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) =>
    request.kind === 'backup_profiles'
      ? []
      : request.kind === 'backup_sync'
        ? readOnlySync(request.request)
        : report('comparing'),
  );
  open();
  const cancel = await screen.findByRole('button', { name: 'Cancel comparison' });
  expect(screen.getByRole('button', { name: 'Compare' })).toBeDisabled();
  fireEvent.click(cancel);
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_compare_cancel', run_id: 'run' },
      'backup_comparison',
    ),
  );
  expect(
    vi.mocked(libraryData).mock.calls.some(([request]) => request.kind === 'backup_compare_start'),
  ).toBe(false);
});
it.each(['identity', 'path', 'availability'])(
  'discards a late response when the base-root %s changes',
  async (changed) => {
    let finish!: (value: ComparisonRun) => void;
    vi.mocked(libraryData).mockImplementation(async (request) =>
      request.kind === 'backup_profiles'
        ? []
        : request.kind === 'backup_sync'
          ? readOnlySync(request.request)
          : request.kind === 'backup_comparison_latest'
            ? null
            : new Promise<ComparisonRun>((resolve) => {
                finish = resolve;
              }),
    );
    const view = open();
    await compare();
    await waitFor(() => expect(finish).toBeDefined());
    view.rerender(
      <BackupScreen
        status={{
          ...status,
          roots: [
            {
              ...status.roots[0]!,
              label: 'Other base',
              ...(changed === 'identity'
                ? { id: 'other' }
                : changed === 'path'
                  ? { displayPath: '/relinked/base' }
                  : { availability: 'offline' }),
            },
          ],
        }}
        navigationControl={null}
        jobsControl={null}
      />,
    );
    finish(report());
    await screen.findByText('Other base');
    expect(screen.queryByText('Read-only comparison finished.')).not.toBeInTheDocument();
  },
);

it('requires explicit scope review before creating a replacement profile binding', async () => {
  const profile = savedProfile();
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) => {
    if (request.kind === 'backup_profiles') return [profile];
    if (request.kind === 'backup_profile_rebind')
      return { ...profile, id: 'new-binding', supersedes: profile.id };
    return fallback(request, kind);
  });
  open();
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Saved comparison' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByRole('combobox', { name: 'Saved comparison' }), {
    target: { value: profile.id },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Review profile rebinding' }));
  expect(screen.getByRole('region', { name: 'Profile rebinding review' })).toHaveTextContent(
    'Previous source: Family',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Choose destination' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Comparison profile name' }), {
    target: { value: 'Reviewed replacement' },
  });
  await waitFor(() => expect(screen.getByRole('button', { name: 'Compare' })).toBeEnabled());
  expect(screen.getByRole('button', { name: 'Save comparison profile' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox', { name: /I reviewed the previous/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Save comparison profile' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'backup_profile_rebind',
        prior_id: profile.id,
        selection_id: 'selection',
        label: 'Reviewed replacement',
        reviewed: true,
      },
      'backup_profile',
    ),
  );
  expect(screen.getByRole('button', { name: 'Review append plan' })).toBeDisabled();
});

it('does not resurrect a stored profile comparison after the same root ID is relinked', async () => {
  const profile = savedProfile();
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (request, kind) => {
    if (request.kind === 'backup_profiles') return [profile];
    if (request.kind === 'backup_comparison_latest') return { ...report(), profileId: profile.id };
    return fallback(request, kind);
  });
  open({ ...status, roots: [{ ...status.roots[0]!, displayPath: '/relinked/same-id' }] });
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Saved comparison' })).toBeEnabled(),
  );
  fireEvent.change(screen.getByRole('combobox', { name: 'Saved comparison' }), {
    target: { value: profile.id },
  });
  await waitFor(() =>
    expect(
      vi
        .mocked(libraryData)
        .mock.calls.filter(([input]) => input.kind === 'backup_comparison_latest').length,
    ).toBeGreaterThan(1),
  );
  expect(screen.queryByText('Read-only comparison finished.')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Review append plan' })).toBeDisabled();
});

it('keeps the inspected comparison entry fixed while a durable transfer is active', async () => {
  let active = false;
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  const job: SyncJob = {
    id: 'active-transfer',
    runId: 'run',
    sourceRootId: 'base',
    action: 'copy_files',
    state: 'running',
    phase: 'copying',
    createdAt: '2026-11-02',
    updatedAt: '2026-11-02',
    sourceDisplayPath: '/generated/source',
    datasetDisplayPath: '/generated/dest',
    totalFiles: 1,
    verifiedFiles: 0,
    copyBytes: '4',
    uncopiedSourceRevisions: 0,
    retainedDestinationFiles: 0,
    readyReviewId: null,
    bytesPerSecond: '10485760',
    message: 'Generated transfer in progress.',
  };
  vi.mocked(libraryData).mockImplementation(async (request, kind) => {
    if (request.kind === 'backup_sync' && request.request.kind === 'list' && active)
      return { kind: 'jobs', data: [job] };
    return fallback(request, kind);
  });
  open();
  await compare();
  const entry = await screen.findByRole('button', { name: '<script>hostile</script>.xmp' });
  fireEvent.click(entry);
  expect(screen.getByRole('region', { name: 'Entry details' })).toBeVisible();
  active = true;
  fireEvent.click(screen.getByRole('button', { name: 'Refresh safe-copy history' }));
  await screen.findByText('Generated transfer in progress.');
  await waitFor(() => expect(entry).toBeDisabled());
  expect(screen.getByRole('region', { name: 'Entry details' })).toBeVisible();
});
