import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import type { LibraryStatus } from '../../bindings';
import { AlbumEditor } from './AlbumEditor';
import { metadata } from '../../metadata/ipc';
import { fixtureAsset } from '../../library/__tests__/fixtures';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
vi.mock('../../library/useAssetFeed', () => ({
  useAssetFeed: () => ({
    page: { assets: [fixtureAsset('a'), fixtureAsset('b')] },
    key: 'fixture',
    busy: false,
  }),
}));
vi.mock('../../library/AssetBrowser', () => ({
  AssetBrowser: ({
    onSelect,
    selected,
  }: {
    onSelect: (id: string) => void;
    selected: ReadonlySet<string>;
  }) => (
    <>
      {['a', 'b'].map((id) => (
        <button type="button" key={id} aria-pressed={selected.has(id)} onClick={() => onSelect(id)}>
          Select {id}
        </button>
      ))}
    </>
  ),
}));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
beforeEach(() => {
  vi.mocked(metadata).mockReset().mockResolvedValue({
    id: 'album',
    name: 'Weekends',
    version: 1,
    kind: 'album',
    archived: false,
    query: null,
    memberCount: 1,
  });
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});
it('creates an album with a distinct selection, including the preselected context photo', async () => {
  const onSaved = vi.fn();
  render(<AlbumEditor status={status} initialIds={['a']} onClose={vi.fn()} onSaved={onSaved} />);
  fireEvent.change(screen.getByLabelText(/Album name/), { target: { value: '  Weekends  ' } });
  expect(screen.getByRole('button', { name: 'Select a' })).toHaveAttribute('aria-pressed', 'true');
  fireEvent.click(screen.getByText('Select b'));
  fireEvent.click(screen.getByText('Select a'));
  fireEvent.click(screen.getByText('Select a'));
  fireEvent.click(screen.getByRole('button', { name: 'Create album' }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
  expect(metadata).toHaveBeenCalledWith(
    { kind: 'create_album', name: 'Weekends', asset_ids: ['b', 'a'] },
    'collection',
  );
});
it('retains name and selection after a native duplicate-name failure', async () => {
  vi.mocked(metadata).mockRejectedValue(new Error('An album with this name already exists.'));
  render(<AlbumEditor status={status} onClose={vi.fn()} onSaved={vi.fn()} />);
  fireEvent.change(screen.getByLabelText(/Album name/), { target: { value: 'Coast' } });
  fireEvent.click(screen.getByText('Select a'));
  fireEvent.click(screen.getByRole('button', { name: 'Create album' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('already exists');
  expect(screen.getByLabelText(/Album name/)).toHaveValue('Coast');
  expect(screen.getByText('Select a')).toHaveAttribute('aria-pressed', 'true');
});
it('adds photos to a custom album using its current version and never replaces memberships', async () => {
  const onSaved = vi.fn();
  render(
    <AlbumEditor
      status={status}
      album={{
        id: 'custom',
        version: 4,
        name: 'Family',
        kind: 'album',
        archived: false,
        query: null,
        memberCount: 30,
      }}
      onClose={vi.fn()}
      onSaved={onSaved}
    />,
  );
  expect(screen.getByRole('button', { name: 'Add photos' })).toBeDisabled();
  fireEvent.click(screen.getByText('Select b'));
  fireEvent.click(screen.getByRole('button', { name: 'Add photos' }));
  await waitFor(() => expect(onSaved).toHaveBeenCalledOnce());
  expect(metadata).toHaveBeenCalledWith(
    {
      kind: 'collection_members',
      id: 'custom',
      expected_version: 4,
      expected_count: 1,
      selection: { kind: 'ids', ids: ['b'] },
      remove: false,
    },
    'membership',
  );
});

it('keeps a pending save open and retains the draft if the catalog rejects it', async () => {
  let rejectSave: (reason: Error) => void = () => {};
  vi.mocked(metadata).mockReturnValueOnce(
    new Promise<never>((_, reject) => {
      rejectSave = reject;
    }),
  );
  const onClose = vi.fn();
  render(<AlbumEditor status={status} onClose={onClose} onSaved={vi.fn()} />);
  fireEvent.change(screen.getByLabelText(/Album name/), { target: { value: 'Weekends' } });
  fireEvent.click(screen.getByRole('button', { name: 'Create album' }));
  expect(screen.getByRole('button', { name: 'Close' })).toBeDisabled();
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(onClose).not.toHaveBeenCalled();
  rejectSave(new Error('Catalog changed. Keep your draft.'));
  expect(await screen.findByRole('alert')).toHaveTextContent('Keep your draft');
  expect(screen.getByRole('dialog')).toHaveAttribute('open');
  expect(screen.getByLabelText(/Album name/)).toHaveValue('Weekends');
});
