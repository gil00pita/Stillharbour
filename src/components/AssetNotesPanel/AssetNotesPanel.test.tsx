import { fireEvent, render, screen, waitFor } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import type { AssetNotes } from '../../bindings';
import { AssetNotesPanel } from '@/components/AssetNotesPanel';
import { metadata } from '../../metadata/ipc';
import { NotesDraftContext, type NotesDraft } from '../../metadata/notesDrafts';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
const notes: AssetNotes = {
  assetId: 'generated',
  version: 3,
  caption: 'Existing caption',
  rejected: false,
  observedKeywords: ['Embedded keyword'],
  keywordsAvailable: true,
  keywordsTruncated: false,
};
beforeEach(() => {
  vi.mocked(metadata)
    .mockReset()
    .mockResolvedValue(notes as never);
});
it('saves catalog captions and rejection against the loaded version with source provenance separate', async () => {
  render(<AssetNotesPanel assetId="generated" />);
  expect(await screen.findByLabelText('Caption')).toHaveValue('Existing caption');
  fireEvent.change(screen.getByLabelText('Caption'), {
    target: { value: '<script>data only</script>\n港' },
  });
  fireEvent.click(screen.getByLabelText('Mark as rejected'));
  vi.mocked(metadata).mockResolvedValueOnce({
    ...notes,
    version: 4,
    caption: '<script>data only</script>\n港',
    rejected: true,
  } as never);
  fireEvent.click(screen.getByRole('button', { name: 'Save caption & review' }));
  await waitFor(() =>
    expect(metadata).toHaveBeenCalledWith(
      {
        kind: 'save_notes',
        asset_id: 'generated',
        expected_version: 3,
        caption: '<script>data only</script>\n港',
        rejected: true,
      },
      'notes',
    ),
  );
  expect(await screen.findByText('Caption and review flag saved in the catalog.')).toBeVisible();
  expect(document.querySelector('script')).toBeNull();
  expect(screen.getByText('Embedded keyword')).toBeInTheDocument();
});
it('retains a conflicting draft and only reloads after an explicit request', async () => {
  render(<AssetNotesPanel assetId="generated" />);
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'My draft' } });
  vi.mocked(metadata).mockRejectedValueOnce(
    new Error('Catalog metadata changed. Reload it before saving.'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Save caption & review' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog metadata changed');
  expect(screen.getByLabelText('Caption')).toHaveValue('My draft');
  vi.mocked(metadata).mockResolvedValueOnce({
    ...notes,
    version: 4,
    caption: 'Newer saved caption',
  } as never);
  fireEvent.click(screen.getByRole('button', { name: 'Reload saved notes' }));
  await waitFor(() => expect(screen.getByLabelText('Caption')).toHaveValue('Newer saved caption'));
});
it('reports read failure without showing editable default values', async () => {
  vi.mocked(metadata).mockRejectedValueOnce(new Error('Catalog unavailable.'));
  render(<AssetNotesPanel assetId="generated" />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable.');
  expect(screen.queryByLabelText('Caption')).not.toBeInTheDocument();
});
it('discards late note reads after changing the selected asset', async () => {
  let late: ((value: AssetNotes) => void) | undefined;
  vi.mocked(metadata).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        late = resolve as (value: AssetNotes) => void;
      }),
  );
  const { rerender } = render(<AssetNotesPanel assetId="first" />);
  rerender(<AssetNotesPanel assetId="second" />);
  expect(await screen.findByLabelText('Caption')).toHaveValue('Existing caption');
  late?.({ ...notes, assetId: 'first', caption: 'Stale first asset' });
  expect(screen.getByLabelText('Caption')).toHaveValue('Existing caption');
});

it('retains drafts across photo navigation and Info remounts without rebasing their saved version', async () => {
  const drafts = new Map<string, NotesDraft>();
  const panel = (id: string | null) => (
    <NotesDraftContext.Provider value={drafts}>
      {id && <AssetNotesPanel assetId={id} />}
    </NotesDraftContext.Provider>
  );
  const { rerender } = render(panel('generated'));
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), {
    target: { value: 'Unsaved coastal memory' },
  });
  fireEvent.click(screen.getByLabelText('Mark as rejected'));
  rerender(panel('second'));
  expect(await screen.findByLabelText('Caption')).toHaveValue('Existing caption');
  rerender(panel(null));
  vi.mocked(metadata).mockResolvedValueOnce({
    ...notes,
    version: 9,
    caption: 'Changed elsewhere',
  } as never);
  rerender(panel('generated'));
  expect(await screen.findByLabelText('Caption')).toHaveValue('Unsaved coastal memory');
  expect(screen.getByLabelText('Mark as rejected')).toBeChecked();
  vi.mocked(metadata).mockRejectedValueOnce(new Error('Catalog metadata changed.'));
  fireEvent.click(screen.getByRole('button', { name: 'Save caption & review' }));
  await screen.findByRole('alert');
  expect(metadata).toHaveBeenLastCalledWith(
    expect.objectContaining({
      kind: 'save_notes',
      expected_version: 3,
      caption: 'Unsaved coastal memory',
    }),
    'notes',
  );
  expect(drafts.get('generated')?.caption).toBe('Unsaved coastal memory');
});

it('does not leak a draft into another catalog with the same asset ID', async () => {
  const first = new Map<string, NotesDraft>();
  const second = new Map<string, NotesDraft>();
  const { rerender } = render(
    <NotesDraftContext.Provider value={first}>
      <AssetNotesPanel assetId="generated" />
    </NotesDraftContext.Provider>,
  );
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'First catalog draft' } });
  rerender(
    <NotesDraftContext.Provider key="other-catalog" value={second}>
      <AssetNotesPanel assetId="generated" />
    </NotesDraftContext.Provider>,
  );
  expect(await screen.findByLabelText('Caption')).toHaveValue('Existing caption');
  expect(second.size).toBe(0);
});

it('keeps a newer draft when an earlier save completes after navigating away', async () => {
  const drafts = new Map<string, NotesDraft>();
  const panel = (visible: boolean) => (
    <NotesDraftContext.Provider value={drafts}>
      {visible && <AssetNotesPanel assetId="generated" />}
    </NotesDraftContext.Provider>
  );
  const { rerender } = render(panel(true));
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'Submitted draft' } });
  let finish: ((value: AssetNotes) => void) | undefined;
  vi.mocked(metadata).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve as (value: AssetNotes) => void;
      }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Save caption & review' }));
  rerender(panel(false));
  rerender(panel(true));
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'Newer draft' } });
  finish?.({ ...notes, version: 4, caption: 'Submitted draft' });
  await waitFor(() => expect(drafts.get('generated')?.caption).toBe('Newer draft'));
  expect(screen.getByLabelText('Caption')).toHaveValue('Newer draft');
});

it('does not discard drafts if explicitly reloading saved notes fails', async () => {
  const drafts = new Map<string, NotesDraft>();
  render(
    <NotesDraftContext.Provider value={drafts}>
      <AssetNotesPanel assetId="generated" />
    </NotesDraftContext.Provider>,
  );
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'Keep this draft' } });
  vi.mocked(metadata).mockRejectedValueOnce(new Error('Catalog unavailable.'));
  fireEvent.click(screen.getByRole('button', { name: 'Reload saved notes' }));
  await screen.findByRole('alert');
  expect(drafts.get('generated')?.caption).toBe('Keep this draft');
});

it('refuses new drafts at the session limit without evicting existing work', async () => {
  const drafts = new Map<string, NotesDraft>(
    Array.from({ length: 240 }, (_, index) => [
      String(index),
      { baseline: notes, caption: `Draft ${index}`, rejected: false },
    ]),
  );
  render(
    <NotesDraftContext.Provider value={drafts}>
      <AssetNotesPanel assetId="generated" />
    </NotesDraftContext.Provider>,
  );
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'Overflow draft' } });
  expect(screen.getByRole('alert')).toHaveTextContent('Save or discard');
  expect(screen.getByLabelText('Caption')).toHaveValue('Existing caption');
  expect(drafts.size).toBe(240);
  expect(drafts.get('0')?.caption).toBe('Draft 0');
});

it('locks the draft during an explicitly requested reload until the saved state arrives', async () => {
  render(<AssetNotesPanel assetId="generated" />);
  await screen.findByLabelText('Caption');
  fireEvent.change(screen.getByLabelText('Caption'), { target: { value: 'Discard this draft' } });
  let finish: ((value: AssetNotes) => void) | undefined;
  vi.mocked(metadata).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        finish = resolve as (value: AssetNotes) => void;
      }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Reload saved notes' }));
  expect(screen.getByLabelText('Caption')).toBeDisabled();
  expect(screen.getByLabelText('Mark as rejected')).toBeDisabled();
  finish?.({ ...notes, version: 4, caption: 'Latest saved notes' });
  await waitFor(() => expect(screen.getByLabelText('Caption')).toHaveValue('Latest saved notes'));
  expect(screen.getByLabelText('Caption')).toBeEnabled();
});
