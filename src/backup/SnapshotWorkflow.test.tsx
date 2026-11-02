import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  LibraryStatus,
  SnapshotCapabilities,
  SnapshotJob,
  SnapshotProfile,
  SnapshotRecord,
  SnapshotReply,
  SnapshotRequest,
  SnapshotReview,
  SnapshotSchedule,
} from '../bindings';
import { libraryData, pickDirectory } from '../ipc';
import { SnapshotWorkflow } from './SnapshotWorkflow';

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
      displayPath: '/generated/base',
      isBase: true,
      availability: 'online',
    },
  ],
  jobs: [],
} as unknown as LibraryStatus;
const capabilities: SnapshotCapabilities = {
  helpersAvailable: true,
  credentialsAvailable: true,
  backupAvailable: true,
  restoreAvailable: true,
  scheduleAvailable: true,
  protocols: ['local', 'sftp', 'ftps', 'ftp', 'smb'],
  unavailableReason: null,
  helperLimits: 'Helpers do not establish physical disk limits.',
  scheduleLimits: 'Exact reviewed manifests only while the process is running.',
};
const profile: SnapshotProfile = {
  id: 'profile',
  label: 'Family history',
  catalogId: 'catalog',
  rootIds: ['base'],
  protocol: 'local',
  destinationDisplay: '/generated/repository',
  repositoryId: 'repository-id',
  createdAt: '2026-11-02T00:00:00Z',
  hostFingerprint: null,
  insecureTransport: false,
  credentialsPresent: true,
  status: 'ready',
};
const record: SnapshotRecord = {
  id: 'snapshot-1',
  profileId: 'profile',
  time: '2026-11-02T00:00:00Z',
  files: 2,
  bytes: '10',
  rootIds: ['base'],
  included: true,
  structuralChecked: false,
  dataReadVerified: false,
  restoredCompared: false,
  catalogIncluded: true,
  sourceRetained: true,
};
function review(action: 'backup' | 'restore' = 'backup'): SnapshotReview {
  return {
    id: `${action}-review`,
    profileId: 'profile',
    action,
    digest: action === 'backup' ? 'a'.repeat(64) : 'b'.repeat(64),
    files: 2,
    bytes: '10',
    rootIds: ['base'],
    sourceDisplayPaths: ['/native-reviewed/base'],
    items: [
      { displayPath: 'photo.bin', bytes: '5', fullDigest: 'f'.repeat(64) },
      { displayPath: 'photo.xmp', bytes: '5', fullDigest: 'e'.repeat(64) },
    ],
    catalogIncluded: true,
    sourceRetained: true,
    targetDisplay:
      action === 'backup' ? '/native-reviewed/repository' : '/native-reviewed/new-restore',
    issues: [],
    executionAvailable: true,
    unavailableReason: null,
  };
}
function job(action = 'backup'): SnapshotJob {
  return {
    id: `${action}-job`,
    profileId: 'profile',
    action,
    state: 'completed_verified',
    phase: 'completed',
    files: 2,
    bytes: '10',
    snapshotId: 'snapshot-1',
    message: 'Native operation finished with the evidence shown below.',
    startedAt: '2026-11-02T00:00:00Z',
    finishedAt: '2026-11-02T00:00:01Z',
    included: true,
    structuralChecked: true,
    dataReadVerified: true,
    restoredCompared: action === 'restore',
    sourceRetained: true,
    independentRecoveryVerified: false,
  };
}
function policy(enabled = true): SnapshotSchedule {
  return {
    profileId: 'profile',
    intervalMinutes: 1440,
    timezone: 'UTC',
    dstPolicy: 'elapsed_interval',
    enabled,
    catchUp: true,
    nextDue: '2026-11-03T00:00:00Z',
    lastRun: null,
    fullyQuitSupported: false,
    approvedDigest: 'a'.repeat(64),
  };
}
let handler: (request: SnapshotRequest) => Promise<SnapshotReply>;
beforeEach(() => {
  handler = async (input) => {
    if (input.kind === 'capabilities') return { kind: 'capabilities', data: capabilities };
    if (input.kind === 'profiles') return { kind: 'profiles', data: [profile] };
    if (input.kind === 'jobs') return { kind: 'jobs', data: [] };
    if (input.kind === 'snapshots') return { kind: 'snapshots', data: [record] };
    if (input.kind === 'schedule_get') return { kind: 'schedule', data: null };
    if (input.kind === 'schedule_disable') return { kind: 'schedule', data: policy(false) };
    if (input.kind === 'profile_open') return { kind: 'profile', data: profile };
    if (input.kind === 'profile_create')
      return {
        kind: 'profile',
        data: {
          ...profile,
          id: 'new-profile',
          label: input.label,
          rootIds: input.root_ids,
          protocol: input.destination.kind,
        },
      };
    if (input.kind === 'backup_review') return { kind: 'review', data: review() };
    if (input.kind === 'restore_review') return { kind: 'review', data: review('restore') };
    if (input.kind === 'backup_approve' || input.kind === 'restore_approve')
      return {
        kind: 'approval',
        data: { id: 'approval', reviewId: input.review_id, digest: input.digest },
      };
    if (input.kind === 'backup_start') return { kind: 'job', data: job() };
    if (input.kind === 'restore_start') return { kind: 'job', data: job('restore') };
    if (input.kind === 'verify') return { kind: 'job', data: job('verify') };
    if (input.kind === 'schedule_save')
      return {
        kind: 'schedule',
        data: {
          ...policy(input.enabled),
          approvedDigest: input.digest,
          intervalMinutes: input.interval_minutes,
          catchUp: input.catch_up,
        },
      };
    if (input.kind === 'browse')
      return {
        kind: 'browse',
        data: {
          profileId: input.profile_id,
          snapshotId: input.snapshot_id,
          entries: [
            {
              id: 'entry',
              displayPath: '<script>alert(1)</script>.mp4',
              kind: 'file',
              bytes: '10',
              digest: null,
            },
          ],
          nextOffset: null,
          total: 1,
        },
      };
    throw new Error(`Unexpected snapshot request ${input.kind}`);
  };
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'native-selection',
    displayPath: '/renderer-display-only',
    identityAvailable: true,
  });
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (input) => {
      if (input.kind === 'backup_snapshots') return handler(input.request);
      if (input.kind === 'backup_credential_enroll')
        return input.purpose === 'repository' ? 'native-repository-ref' : 'native-transport-ref';
      if (input.kind === 'backup_credential_export')
        return 'Recovery key written into the new selected file.';
      if (input.kind === 'backup_credential_unlock')
        return { kind: 'profile', data: { ...profile, id: 'unlocked-profile' } };
      throw new Error('Unexpected IPC family');
    });
});
function open(value = status) {
  return render(<SnapshotWorkflow status={value} onActivityChange={vi.fn()} />);
}
async function selectProfile() {
  const select = await screen.findByRole('combobox', { name: 'Snapshot profile' });
  await waitFor(() => expect(select).toBeEnabled());
  fireEvent.change(select, { target: { value: 'profile' } });
  await screen.findByRole('region', { name: 'Snapshot profile workspace' });
  await screen.findAllByText('Snapshot snapshot-1');
}
async function backupReview() {
  fireEvent.click(screen.getByRole('button', { name: 'Review backup scope' }));
  return screen.findByRole('region', { name: 'Snapshot action review' });
}

it('creates a profile from the fixed base and opaque native refs without a password field or automatic backup', async () => {
  open();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'New snapshot profile' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'New snapshot profile' }));
  const form = screen.getByRole('region', { name: 'New snapshot profile' });
  expect(within(form).queryByRole('button', { name: /choose source/i })).not.toBeInTheDocument();
  expect(document.querySelector('input[type="password"]')).toBeNull();
  fireEvent.change(within(form).getByRole('textbox', { name: 'Snapshot profile name' }), {
    target: { value: 'New backup' },
  });
  fireEvent.click(within(form).getByRole('button', { name: 'Choose snapshot destination' }));
  fireEvent.click(
    await within(form).findByRole('button', { name: 'Set repository recovery password…' }),
  );
  await waitFor(() =>
    expect(within(form).getByRole('button', { name: 'Save snapshot profile' })).toBeEnabled(),
  );
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'backup_credential_enroll', purpose: 'repository', destination: null },
    'backup_credential_ref',
  );
  fireEvent.click(within(form).getByRole('button', { name: 'Save snapshot profile' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'backup_snapshots',
        request: {
          kind: 'profile_create',
          label: 'New backup',
          root_ids: ['base'],
          repository_credential_ref: 'native-repository-ref',
          transport_credential_ref: null,
          destination: { kind: 'local', selection_id: 'native-selection' },
        },
      },
      'backup_snapshots',
    ),
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) =>
          input.kind === 'backup_snapshots' &&
          (input.request.kind === 'backup_review' || input.request.kind === 'backup_start'),
      ),
  ).toBe(false);
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Snapshot profile' })).toBeEnabled(),
  );
});

it('requires explicit insecure FTP acceptance and drops enrolled remote refs when the endpoint changes', async () => {
  open();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'New snapshot profile' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'New snapshot profile' }));
  const form = screen.getByRole('region', { name: 'New snapshot profile' });
  fireEvent.change(within(form).getByRole('combobox', { name: 'Snapshot transport' }), {
    target: { value: 'ftp' },
  });
  fireEvent.change(within(form).getByRole('textbox', { name: 'Server host' }), {
    target: { value: 'fixture.local' },
  });
  fireEvent.change(within(form).getByRole('textbox', { name: 'Remote username' }), {
    target: { value: 'fixture' },
  });
  expect(within(form).getByRole('button', { name: 'Set remote credentials…' })).toBeDisabled();
  fireEvent.click(
    within(form).getByRole('checkbox', { name: /explicitly accept this unencrypted FTP/ }),
  );
  fireEvent.click(within(form).getByRole('button', { name: 'Set remote credentials…' }));
  await screen.findByText(/Remote credentials saved for these exact endpoint settings/);
  fireEvent.change(within(form).getByRole('textbox', { name: 'Server host' }), {
    target: { value: 'other.local' },
  });
  expect(
    screen.queryByText(/Remote credentials saved for these exact endpoint settings/),
  ).not.toBeInTheDocument();
  expect(within(form).getByRole('button', { name: 'Save snapshot profile' })).toBeDisabled();
  const call = vi
    .mocked(libraryData)
    .mock.calls.find(
      ([input]) => input.kind === 'backup_credential_enroll' && input.purpose === 'transport',
    );
  expect(call?.[0]).toEqual({
    kind: 'backup_credential_enroll',
    purpose: 'transport',
    destination: {
      kind: 'ftp',
      host: 'fixture.local',
      port: 21,
      user: 'fixture',
      remote_path: '',
      insecure_acknowledged: true,
    },
  });
});

it('reopens a local repository only through a fresh native selection', async () => {
  open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Reauthorize repository parent…' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'backup_snapshots',
        request: { kind: 'profile_open', profile_id: 'profile', selection_id: 'native-selection' },
      },
      'backup_snapshots',
    ),
  );
  expect(pickDirectory).toHaveBeenCalledTimes(1);
});

it('requires a separate exact backup approval and never promotes inclusion into data or restore verification', async () => {
  open();
  await selectProfile();
  const card = screen.getByText('Snapshot snapshot-1').closest('li')!;
  expect(
    within(card).getByText('Stored data read and verified').nextElementSibling,
  ).toHaveTextContent('Not established');
  expect(within(card).getByText('Restored and byte-compared').nextElementSibling).toHaveTextContent(
    'Not established',
  );
  const panel = await backupReview();
  expect(within(panel).getByText('/native-reviewed/base')).toBeVisible();
  expect(
    within(panel).getByRole('button', { name: 'Approve this snapshot action' }),
  ).toBeDisabled();
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
  await screen.findByText('Native operation finished with the evidence shown below.');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: { kind: 'backup_start', review_id: 'backup-review', approval_id: 'approval' },
    },
    'backup_snapshots',
  );
});

it('renders repository filenames as data and reviews a whole-snapshot restore using only a native target ID', async () => {
  open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Browse snapshot' }));
  const browser = await screen.findByRole('region', { name: 'Snapshot browser' });
  expect(await within(browser).findByText('<script>alert(1)</script>.mp4')).toBeVisible();
  expect(browser.querySelector('script')).toBeNull();
  expect(within(browser).getByText(/paging does not change that scope/)).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Review restore to new output…' }));
  const panel = await screen.findByRole('region', { name: 'Snapshot action review' });
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'restore_review',
        profile_id: 'profile',
        snapshot_id: 'snapshot-1',
        selection_id: 'native-selection',
      },
    },
    'backup_snapshots',
  );
  expect(within(panel).getByText(/does not overwrite an active library/)).toBeVisible();
  expect(
    screen.queryByRole('button', { name: 'Start restore to new output' }),
  ).not.toBeInTheDocument();
});

it('binds recurring consent to the current exact review and never enables fully-quit automation', async () => {
  open();
  await selectProfile();
  let panel = screen.getByRole('region', { name: 'Snapshot schedule' });
  expect(within(panel).getByRole('button', { name: 'Save snapshot schedule' })).toBeDisabled();
  await backupReview();
  panel = screen.getByRole('region', { name: 'Snapshot schedule' });
  fireEvent.click(within(panel).getByRole('checkbox', { name: /Enable repeats of this exact/ }));
  fireEvent.click(
    within(panel).getByRole('checkbox', { name: /approve repeating only this reviewed manifest/ }),
  );
  fireEvent.click(within(panel).getByRole('button', { name: 'Save snapshot schedule' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'backup_snapshots',
        request: {
          kind: 'schedule_save',
          profile_id: 'profile',
          review_id: 'backup-review',
          digest: 'a'.repeat(64),
          interval_minutes: 1440,
          timezone: 'UTC',
          enabled: true,
          catch_up: true,
        },
      },
      'backup_snapshots',
    ),
  );
  expect(within(panel).getByText(/Fully-quit execution is unavailable/)).toBeVisible();
  expect(within(panel).getByText(/New or changed files/)).toBeVisible();
});

it('labels the schedule authority timestamp as an attempt without claiming a completed run', async () => {
  const normal = handler;
  handler = async (input) =>
    input.kind === 'schedule_get'
      ? { kind: 'schedule', data: { ...policy(), lastRun: '2026-11-02T01:02:03Z' } }
      : normal(input);
  open();
  await selectProfile();
  const panel = screen.getByRole('region', { name: 'Snapshot schedule' });
  expect(within(panel).getByText(/Last scheduled attempt 2026-11-02T01:02:03Z/)).toBeVisible();
  expect(within(panel).queryByText(/Last run/)).not.toBeInTheDocument();
});

it('allows disabling future repeats without a fresh source review when the source is offline', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'capabilities'
      ? {
          kind: 'capabilities',
          data: { ...capabilities, backupAvailable: false, scheduleAvailable: false },
        }
      : input.kind === 'schedule_get'
        ? { kind: 'schedule', data: policy() }
        : fallback(input);
  open({ ...status, roots: [{ ...status.roots[0]!, availability: 'offline' }] });
  await selectProfile();
  fireEvent.click(await screen.findByRole('button', { name: 'Disable future snapshot repeats' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_snapshots', request: { kind: 'schedule_disable', profile_id: 'profile' } },
      'backup_snapshots',
    ),
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'backup_review',
      ),
  ).toBe(false);
});

it('drops a delayed review when the same root ID is relinked to a different displayed path', async () => {
  let finish!: (reply: SnapshotReply) => void;
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'backup_review'
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : fallback(input);
  const view = open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Review backup scope' }));
  await waitFor(() => expect(finish).toBeDefined());
  view.rerender(
    <SnapshotWorkflow
      status={{
        ...status,
        roots: [{ ...status.roots[0]!, displayPath: '/different/native-root' }],
      }}
      onActivityChange={vi.fn()}
    />,
  );
  finish({ kind: 'review', data: review() });
  await screen.findByText('/different/native-root');
  expect(screen.queryByRole('region', { name: 'Snapshot action review' })).not.toBeInTheDocument();
});

it('discards a digest-mismatched approval instead of exposing a start action', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'backup_approve'
      ? { kind: 'approval', data: { id: 'foreign', reviewId: 'backup-review', digest: 'wrong' } }
      : fallback(input);
  open();
  await selectProfile();
  const panel = await backupReview();
  fireEvent.click(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ }));
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this snapshot action' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Approval does not match this snapshot review',
  );
  expect(screen.queryByRole('button', { name: 'Start encrypted backup' })).not.toBeInTheDocument();
});

it('consumes a snapshot approval after an uncertain start and requires history inspection', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'backup_start') throw new Error('Native reply was lost.');
    return fallback(input);
  };
  open();
  await selectProfile();
  const panel = await backupReview();
  fireEvent.click(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ }));
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this snapshot action' }));
  fireEvent.click(await within(panel).findByRole('button', { name: 'Start encrypted backup' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Refresh snapshot history');
  expect(screen.queryByRole('region', { name: 'Snapshot action review' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Start encrypted backup' })).not.toBeInTheDocument();
  expect(
    screen.queryByText('Native operation finished with the evidence shown below.'),
  ).not.toBeInTheDocument();
  const starts = vi
    .mocked(libraryData)
    .mock.calls.filter(
      ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'backup_start',
    );
  expect(starts).toHaveLength(1);
});

it('unlocks through native IPC into a new profile and retires the old action approval', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'snapshots' && input.profile_id === 'unlocked-profile')
      return { kind: 'snapshots', data: [{ ...record, profileId: 'unlocked-profile' }] };
    return fallback(input);
  };
  open();
  await selectProfile();
  const panel = await backupReview();
  fireEvent.click(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ }));
  fireEvent.click(within(panel).getByRole('button', { name: 'Approve this snapshot action' }));
  await within(panel).findByRole('button', { name: 'Start encrypted backup' });
  fireEvent.click(screen.getByText('Repository password recovery'));
  fireEvent.click(screen.getByRole('button', { name: 'Unlock repository password…' }));
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Snapshot profile' })).toHaveValue(
      'unlocked-profile',
    ),
  );
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'backup_credential_unlock', profile_id: 'profile', purpose: 'repository' },
    'backup_snapshots',
  );
  expect(screen.queryByRole('button', { name: 'Start encrypted backup' })).not.toBeInTheDocument();
  expect(document.querySelector('input[type="password"]')).toBeNull();
  expect(
    within(screen.getByRole('combobox', { name: 'Snapshot profile' })).getAllByRole('option'),
  ).toHaveLength(3);
});

it('accepts native-verified repository identity when the previously loaded profile had not learned it yet', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'profiles')
      return { kind: 'profiles', data: [{ ...profile, repositoryId: null }] };
    if (input.kind === 'snapshots' && input.profile_id === 'unlocked-profile')
      return { kind: 'snapshots', data: [{ ...record, profileId: 'unlocked-profile' }] };
    return fallback(input);
  };
  open();
  await selectProfile();
  fireEvent.click(screen.getByText('Repository password recovery'));
  fireEvent.click(screen.getByRole('button', { name: 'Unlock repository password…' }));
  await waitFor(() =>
    expect(screen.getByRole('combobox', { name: 'Snapshot profile' })).toHaveValue(
      'unlocked-profile',
    ),
  );
  expect(screen.getByText('Repository identity: repository-id', { exact: false })).toBeVisible();
});

it('refuses an unlocked profile that changes a repository identity already known to the UI', async () => {
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (input, kind) =>
    input.kind === 'backup_credential_unlock'
      ? {
          kind: 'profile',
          data: { ...profile, id: 'foreign-profile', repositoryId: 'different-repository' },
        }
      : fallback(input, kind),
  );
  open();
  await selectProfile();
  fireEvent.click(screen.getByText('Repository password recovery'));
  fireEvent.click(screen.getByRole('button', { name: 'Unlock repository password…' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'does not preserve the reviewed repository',
  );
  expect(screen.getByRole('combobox', { name: 'Snapshot profile' })).toHaveValue('profile');
});

it('rejects a review whose source scope differs from its immutable profile', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'backup_review'
      ? { kind: 'review', data: { ...review(), rootIds: ['unrelated-root'] } }
      : fallback(input);
  open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Review backup scope' }));
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'belongs to another profile or action',
  );
  expect(screen.queryByRole('region', { name: 'Snapshot action review' })).not.toBeInTheDocument();
});

it('retires an in-flight profile enrollment when the source path changes under its ID', async () => {
  let finish!: (reference: string) => void;
  const fallback = vi.mocked(libraryData).getMockImplementation()!;
  vi.mocked(libraryData).mockImplementation(async (input, kind) =>
    input.kind === 'backup_credential_enroll'
      ? new Promise<string>((resolve) => {
          finish = resolve;
        })
      : fallback(input, kind),
  );
  const view = open();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'New snapshot profile' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'New snapshot profile' }));
  fireEvent.click(screen.getByRole('button', { name: 'Set repository recovery password…' }));
  await waitFor(() => expect(finish).toBeDefined());
  view.rerender(
    <SnapshotWorkflow
      status={{ ...status, roots: [{ ...status.roots[0]!, displayPath: '/relinked/base' }] }}
      onActivityChange={vi.fn()}
    />,
  );
  finish('retired-credential-ref');
  await screen.findByText('/relinked/base');
  expect(
    screen.queryByText("Repository recovery password saved in this device's credential vault."),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Save snapshot profile' })).toBeDisabled();
});

it('inspects an interrupted existing snapshot by concrete ID without retrying its backup', async () => {
  const fallback = handler;
  const interrupted = {
    ...job(),
    state: 'needs_recovery',
    snapshotId: null,
    message: 'Interrupted before verification.',
    included: false,
    structuralChecked: false,
    dataReadVerified: false,
    restoredCompared: false,
  };
  handler = async (input) => {
    if (input.kind === 'jobs') return { kind: 'jobs', data: [interrupted] };
    if (input.kind === 'reconcile')
      return {
        kind: 'job',
        data: {
          ...job(),
          id: 'reconciliation-job',
          action: 'reconcile',
          message: 'Existing snapshot inspected and verified.',
        },
      };
    return fallback(input);
  };
  open();
  await selectProfile();
  const panel = screen.getByRole('region', { name: 'Inspect interrupted snapshot' });
  const id = within(panel).getByRole('textbox', { name: 'Existing snapshot ID (optional)' });
  fireEvent.change(id, { target: { value: 'not-an-id' } });
  expect(within(panel).getByRole('button', { name: 'Inspect existing snapshot' })).toBeDisabled();
  fireEvent.change(id, { target: { value: 'a'.repeat(64) } });
  fireEvent.click(within(panel).getByRole('button', { name: 'Inspect existing snapshot' }));
  await screen.findByText('Existing snapshot inspected and verified.');
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'reconcile',
        profile_id: 'profile',
        job_id: 'backup-job',
        snapshot_id: 'a'.repeat(64),
      },
    },
    'backup_snapshots',
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'backup_start',
      ),
  ).toBe(false);
});

it('keeps recovery unverified when an existing snapshot inspection fails', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'jobs')
      return {
        kind: 'jobs',
        data: [
          {
            ...job(),
            state: 'needs_recovery',
            snapshotId: null,
            message: 'Recovery is unresolved.',
            restoredCompared: false,
          },
        ],
      };
    if (input.kind === 'reconcile')
      throw new Error('More than one snapshot matches. Choose a concrete ID.');
    return fallback(input);
  };
  open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Inspect existing snapshot' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('More than one snapshot matches');
  expect(screen.getByText('Recovery is unresolved.')).toBeVisible();
  expect(screen.queryByText('Existing snapshot inspected and verified.')).not.toBeInTheDocument();
  expect(screen.getByRole('region', { name: 'Inspect interrupted snapshot' })).toBeVisible();
});

it('requires a fresh new-output review for an interrupted restore instead of offering backup reconciliation', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'jobs'
      ? {
          kind: 'jobs',
          data: [
            {
              ...job('restore'),
              state: 'needs_recovery',
              restoredCompared: false,
              message: 'Partial restore retained for inspection.',
            },
          ],
        }
      : fallback(input);
  open();
  await selectProfile();
  await screen.findByText('Partial restore retained for inspection.');
  expect(
    screen.queryByRole('region', { name: 'Inspect interrupted snapshot' }),
  ).not.toBeInTheDocument();
  const fresh = screen.getByRole('button', { name: 'Review restore to new output…' });
  expect(fresh).toBeEnabled();
  fireEvent.click(fresh);
  const reviewPanel = await screen.findByRole('region', { name: 'Snapshot action review' });
  expect(
    within(reviewPanel).getByRole('button', { name: 'Approve this snapshot action' }),
  ).toBeDisabled();
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'reconcile',
      ),
  ).toBe(false);
});

it('keeps a stored schedule independently available for disablement when history cannot be read', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'jobs' || input.kind === 'snapshots')
      throw new Error('History read is unavailable.');
    if (input.kind === 'schedule_get') return { kind: 'schedule', data: policy() };
    return fallback(input);
  };
  open({ ...status, roots: [{ ...status.roots[0]!, availability: 'offline' }] });
  const select = await screen.findByRole('combobox', { name: 'Snapshot profile' });
  await waitFor(() => expect(select).toBeEnabled());
  fireEvent.change(select, { target: { value: 'profile' } });
  expect(await screen.findByRole('alert')).toHaveTextContent('History read is unavailable.');
  fireEvent.click(await screen.findByRole('button', { name: 'Disable future snapshot repeats' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_snapshots', request: { kind: 'schedule_disable', profile_id: 'profile' } },
      'backup_snapshots',
    ),
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.some(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'backup_review',
      ),
  ).toBe(false);
});

it('can explicitly disable a stored policy even when effective authorization is unavailable', async () => {
  const fallback = handler;
  handler = async (input) =>
    input.kind === 'schedule_get' ? { kind: 'schedule', data: policy(false) } : fallback(input);
  open();
  await selectProfile();
  expect(
    screen.getByText('Not enabled or authorization unavailable', { exact: false }),
  ).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Disable future snapshot repeats' }));
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      { kind: 'backup_snapshots', request: { kind: 'schedule_disable', profile_id: 'profile' } },
      'backup_snapshots',
    ),
  );
});

it.each(['immediate', 'polled'])(
  'refreshes authoritative repository identity after %s native completion',
  async (mode) => {
    const fallback = handler;
    let initialized = false;
    handler = async (input) => {
      if (input.kind === 'profiles')
        return {
          kind: 'profiles',
          data: [
            {
              ...profile,
              repositoryId: initialized ? profile.repositoryId : null,
              status: initialized ? 'ready' : 'not_initialized',
            },
          ],
        };
      if (input.kind === 'backup_start') {
        initialized = mode === 'immediate';
        return {
          kind: 'job',
          data:
            mode === 'immediate' ? job() : { ...job(), state: 'running', phase: 'snapshotting' },
        };
      }
      if (input.kind === 'job') {
        initialized = true;
        return { kind: 'job', data: job() };
      }
      return fallback(input);
    };
    const view = open();
    await selectProfile();
    expect(
      screen.getByText('Repository identity: Not established yet', { exact: false }),
    ).toBeVisible();
    const panel = await backupReview();
    fireEvent.click(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ }));
    fireEvent.click(within(panel).getByRole('button', { name: 'Approve this snapshot action' }));
    const start = await within(panel).findByRole('button', { name: 'Start encrypted backup' });
    vi.useFakeTimers();
    try {
      await act(async () => fireEvent.click(start));
      if (mode === 'polled') await act(async () => vi.advanceTimersByTimeAsync(500));
      expect(
        screen.getByText('Repository identity: repository-id', { exact: false }),
      ).toBeVisible();
      expect(
        screen.queryByText('Repository identity: Not established yet', { exact: false }),
      ).not.toBeInTheDocument();
      expect(screen.getByText('Last observed access: online', { exact: false })).toBeVisible();
    } finally {
      view.unmount();
      vi.useRealTimers();
    }
  },
);

it('refreshes live profile metadata when history is explicitly refreshed after external native work', async () => {
  const fallback = handler;
  let initialized = false;
  handler = async (input) =>
    input.kind === 'profiles'
      ? {
          kind: 'profiles',
          data: [{ ...profile, repositoryId: initialized ? profile.repositoryId : null }],
        }
      : fallback(input);
  open();
  await selectProfile();
  expect(
    screen.getByText('Repository identity: Not established yet', { exact: false }),
  ).toBeVisible();
  initialized = true;
  fireEvent.click(screen.getByRole('button', { name: 'Refresh snapshot history' }));
  expect(
    await screen.findByText('Repository identity: repository-id', { exact: false }),
  ).toBeVisible();
});

it('reviews a private OLD snapshot using its recorded roots rather than the repository profile ordinary roots', async () => {
  const fallback = handler;
  handler = async (input) => {
    if (input.kind === 'snapshots')
      return { kind: 'snapshots', data: [{ ...record, rootIds: ['old-dataset'] }] };
    if (input.kind === 'restore_review')
      return { kind: 'review', data: { ...review('restore'), rootIds: ['old-dataset'] } };
    return fallback(input);
  };
  open();
  await selectProfile();
  fireEvent.click(screen.getByRole('button', { name: 'Review restore to new output…' }));
  const panel = await screen.findByRole('region', { name: 'Snapshot action review' });
  expect(within(panel).getByRole('checkbox', { name: /reviewed this exact scope/ })).toBeEnabled();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'backup_snapshots',
      request: {
        kind: 'restore_review',
        profile_id: 'profile',
        snapshot_id: 'snapshot-1',
        selection_id: 'native-selection',
      },
    },
    'backup_snapshots',
  );
});

it('discovers scheduled work through bounded metadata refreshes without overlapping a pending request', async () => {
  const view = open();
  await selectProfile();
  const fallback = handler;
  let hold = false;
  let finish!: (value: SnapshotReply) => void;
  handler = async (input) => {
    if (input.kind === 'schedule_get') return { kind: 'schedule', data: policy() };
    if (input.kind === 'jobs' && hold)
      return new Promise((resolve) => {
        finish = resolve;
      });
    return fallback(input);
  };
  vi.useFakeTimers();
  try {
    await act(async () => {
      fireEvent.click(screen.getByRole('button', { name: 'Refresh snapshot history' }));
    });
    const before = vi
      .mocked(libraryData)
      .mock.calls.filter(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'jobs',
      ).length;
    hold = true;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(finish).toBeDefined();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20000);
    });
    const pending = vi
      .mocked(libraryData)
      .mock.calls.filter(
        ([input]) => input.kind === 'backup_snapshots' && input.request.kind === 'jobs',
      ).length;
    expect(pending).toBe(before + 1);
    await act(async () =>
      finish({
        kind: 'jobs',
        data: [
          {
            ...job(),
            state: 'running',
            phase: 'snapshotting',
            snapshotId: null,
            message: 'A scheduled backup was discovered.',
          },
        ],
      }),
    );
    expect(screen.getByText('A scheduled backup was discovered.')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Cancel snapshot job' })).toBeEnabled();
  } finally {
    view.unmount();
    vi.useRealTimers();
  }
});

it.each(['pending', 'failed'] as const)(
  'shows the local retained action independently of %s snapshot profiles and unavailable helpers',
  async (profileState) => {
    const normal = handler;
    handler = async (request) => {
      if (request.kind === 'capabilities')
        return {
          kind: 'capabilities',
          data: {
            ...capabilities,
            latestRetainedCatalogTransactionId: 'latest-native-transaction',
            helpersAvailable: false,
            credentialsAvailable: false,
            backupAvailable: false,
            restoreAvailable: false,
            scheduleAvailable: false,
            unavailableReason: 'Snapshot helpers are unavailable',
          },
        };
      if (request.kind === 'profiles') {
        if (profileState === 'failed') throw new Error('Repository credentials unavailable');
        return await new Promise<SnapshotReply>(() => {});
      }
      if (request.kind === 'catalog_restore_review_retained')
        return {
          kind: 'catalog_restore_review',
          data: {
            id: 'local-retained-review',
            digest: 'a'.repeat(64),
            sourceKind: 'retained_pre_restore',
            restoreJobId: null,
            snapshotId: null,
            retainedTransactionId: 'latest-native-transaction',
            catalogId: profile.catalogId,
            schema: 17,
            currentRevision: 'current-local',
            restoredRevision: 'prior-local',
            roots: [],
            approvalAvailable: false,
            warning: 'Current edits will be preserved separately',
          },
        };
      return normal(request);
    };
    render(
      <SnapshotWorkflow status={status} catalogId={profile.catalogId} onActivityChange={vi.fn()} />,
    );
    const button = await screen.findByRole('button', { name: 'Review latest retained catalog' });
    expect(button).toBeEnabled();
    fireEvent.click(button);
    await screen.findByText('current-local');
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
    expect(screen.getByRole('combobox', { name: 'Snapshot profile' })).toHaveValue('');
  },
);
