import { fireEvent, render, screen, waitFor, within } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { OrganizationPreview as Preview } from '../../bindings';
import { libraryData } from '../../ipc';
import { OrganizationPreview } from '@/components/OrganizationPreview';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));

function response(): Preview {
  return {
    policy: 'organization-preview-v1',
    fingerprint: 'fixture-fingerprint',
    catalogRevision: '1',
    template: { pattern: '{original_stem}.{ext}', event: '', missingDate: 'undated' },
    rows: [
      {
        assetId: 'photo-id',
        rootId: 'root-id',
        sourcePath: 'Source/<photo>.JPG',
        sourceRevision: 'revision',
        targetRelativePath: 'Undated/<photo>.JPG',
        sequence: 1,
        bytes: '42',
        issues: [],
        usedUndated: true,
        companionCandidates: 2,
      },
    ],
    proposed: 1,
    blocked: 0,
    selectedBytes: '42',
    executionAvailable: false,
  };
}
beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
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
  vi.mocked(libraryData).mockReset().mockResolvedValue(response());
});
function setup() {
  const props = {
    assetIds: ['photo-id'],
    revision: '1',
    roots: [{ id: 'root-id', label: 'Generated library' }],
    onClose: vi.fn(),
  };
  return { ...render(<OrganizationPreview {...props} />), props };
}
async function build() {
  fireEvent.click(screen.getByRole('button', { name: 'Build preview' }));
  await screen.findByText('1 proposed · 0 blocked · 42 selected file bytes');
  await waitFor(() => {
    for (const radio of screen.getAllByRole('radio')) expect(radio).toBeEnabled();
  });
}

it('sends catalog IDs and typed settings, renders untrusted paths as text and keeps execution unavailable', async () => {
  setup();
  await build();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'organization_preview',
      asset_ids: ['photo-id'],
      template: {
        pattern: '{capture:%Y}/{capture:%Y-%m-%d}/{original_stem}.{ext}',
        event: '',
        missingDate: 'undated',
      },
    },
    'organization_preview',
  );
  expect(within(screen.getByRole('table')).getByText('Source/<photo>.JPG')).toBeVisible();
  expect(screen.getByText('Generated library')).toHaveAttribute('title', 'Library ID: root-id');
  expect(screen.getByText('Undated/<photo>.JPG')).toBeVisible();
  expect(screen.getByText(/2 indexed companion candidates/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Create organized copy' })).toBeDisabled();
  expect(document.querySelector('photo')).toBeNull();
  await waitFor(() =>
    expect(
      screen.getByText('Selected photos: current path, proposed relative path and review issues'),
    ).toHaveFocus(),
  );
  fireEvent.change(screen.getByRole('searchbox', { name: 'Find a path' }), {
    target: { value: 'absent' },
  });
  expect(screen.getByText('No paths match your search.')).toBeVisible();
});

it('invalidates prior results when editing policy, pattern or preset', async () => {
  setup();
  await build();
  fireEvent.click(screen.getByRole('radio', { name: 'By month' }));
  await waitFor(() => expect(screen.queryByRole('table')).not.toBeInTheDocument());
  await build();
  fireEvent.click(
    within(screen.getByRole('radiogroup', { name: 'When capture date is missing' })).getByRole(
      'radio',
      { name: 'Block photo' },
    ),
  );
  await waitFor(() => expect(screen.queryByRole('table')).not.toBeInTheDocument());
  await build();
  fireEvent.change(screen.getByLabelText('Path template'), {
    target: { value: '{capture:%Y}/Archive/{original_stem}.{ext}' },
  });
  expect(screen.getByRole('combobox', { name: 'Folder levels' })).toBeVisible();
  await waitFor(() => expect(screen.queryByRole('table')).not.toBeInTheDocument());
});
it('binds explicit stem case to a fresh preview and preserves extension help', async () => {
  setup();
  expect(screen.getByRole('radio', { name: 'Preserve' })).toBeChecked();
  await build();
  fireEvent.click(screen.getByRole('radio', { name: 'UPPERCASE' }));
  await waitFor(() => expect(screen.queryByRole('table')).not.toBeInTheDocument());
  await build();
  expect(libraryData).toHaveBeenCalledWith(
    expect.objectContaining({
      kind: 'organization_preview',
      template: expect.objectContaining({ filenameStemCase: 'uppercase' }),
    }),
    'organization_preview',
  );
  expect(screen.getByText(/Folder names and extension case stay as previewed/)).toBeVisible();
  fireEvent.click(screen.getByRole('radio', { name: 'Preserve' }));
  await waitFor(() => expect(screen.queryByRole('table')).not.toBeInTheDocument());
  await build();
  const last = vi
    .mocked(libraryData)
    .mock.calls.filter(([request]) => request.kind === 'organization_preview')
    .at(-1)?.[0];
  expect(last?.kind).toBe('organization_preview');
  if (last?.kind === 'organization_preview')
    expect(last.template).not.toHaveProperty('filenameStemCase');
});

it('shows a real backend failure and allows a fresh attempt without stale success', async () => {
  setup();
  vi.mocked(libraryData).mockRejectedValueOnce({
    code: 'INVALID_INPUT',
    message: 'Unknown template token.',
  });
  fireEvent.click(screen.getByRole('button', { name: 'Build preview' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Unknown template token.');
  expect(screen.queryByRole('table')).not.toBeInTheDocument();
  await build();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});

it('marks a catalog revision change as stale even while a snapshot request is pending', async () => {
  let resolve: (value: Preview) => void = () => {
    throw new Error('No pending request');
  };
  vi.mocked(libraryData).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const { rerender, props } = setup();
  fireEvent.click(screen.getByRole('button', { name: 'Build preview' }));
  expect(screen.getByLabelText('Path template')).toBeDisabled();
  rerender(<OrganizationPreview {...props} revision="2" />);
  resolve(response());
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'The catalog changed. Build a fresh preview',
  );
  expect(screen.getByRole('button', { name: 'Create organized copy' })).toBeDisabled();
});

it('shows blocked collision participants without suppressing their exact proposed targets', async () => {
  const value = response();
  value.blocked = 1;
  value.proposed = 0;
  value.rows = value.rows.map((row) => ({
    ...row,
    issues: ['collision', 'possible_case_collision'],
  }));
  vi.mocked(libraryData).mockResolvedValueOnce(value);
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'Build preview' }));
  expect(await screen.findByText('Proposed paths collide')).toBeVisible();
  expect(screen.getByText(/may collide on a case-insensitive/)).toBeVisible();
  expect(screen.getByText('Undated/<photo>.JPG')).toBeVisible();
});

it('closes with Escape and restores focus to its gallery opener', async () => {
  const opener = document.createElement('button');
  document.body.append(opener);
  opener.focus();
  const { props, unmount } = setup();
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(props.onClose).toHaveBeenCalledOnce();
  unmount();
  await waitFor(() => expect(opener).toHaveFocus());
  opener.remove();
});
