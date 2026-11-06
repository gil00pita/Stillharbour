import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import { MetadataExportPanel } from '@/components/MetadataExportPanel';
import { library, libraryData, pickDirectory } from '../../ipc';
vi.mock('../../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const review = {
  id: 'review-id',
  digest: 'review-digest',
  revision: '9',
  exportKind: 'catalog',
  destination: 'Generated export folder',
  files: [
    { name: 'stillharbour-catalog-generated.stillharbour', bytes: '100', digest: 'file-digest' },
  ],
  message: 'Unencrypted catalog; no originals, credentials or root permissions.',
};
beforeEach(() => {
  vi.mocked(pickDirectory).mockReset().mockResolvedValue({
    id: 'native-selection',
    displayPath: 'Generated export folder',
    identityAvailable: true,
  });
  vi.mocked(libraryData)
    .mockReset()
    .mockResolvedValue(review as never);
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
});
function panel() {
  return render(
    <MetadataExportPanel
      selection={{ kind: 'ids', ids: ['asset'] }}
      selectionCount={1}
      onBusy={vi.fn()}
    />,
  );
}
it('requires an explicit privacy review and sends only backend IDs/digest to save', async () => {
  panel();
  fireEvent.click(screen.getByRole('button', { name: 'Choose export folder & preview' }));
  const save = await screen.findByRole('button', { name: 'Save reviewed export' });
  expect(save).toBeDisabled();
  expect(libraryData).toHaveBeenCalledWith(
    {
      kind: 'metadata_export_preview',
      export_kind: 'catalog',
      selection: null,
      expected_count: 0,
      selection_id: 'native-selection',
    },
    'metadata_export_review',
  );
  fireEvent.click(screen.getByRole('checkbox'));
  vi.mocked(libraryData).mockResolvedValueOnce({
    id: 'review-id',
    files: 1,
    destination: 'Generated export folder',
    message: 'Verified new output.',
  } as never);
  fireEvent.click(save);
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'metadata_export_save',
        id: 'review-id',
        digest: 'review-digest',
        selection_id: 'native-selection',
      },
      'metadata_export_saved',
    ),
  );
  expect(await screen.findByText(/1 new files saved/)).toBeVisible();
});
it('does not prepare or claim success after picker cancellation', async () => {
  vi.mocked(pickDirectory).mockResolvedValueOnce(null);
  panel();
  fireEvent.click(screen.getByRole('button', { name: 'Choose export folder & preview' }));
  expect(await screen.findByText(/selection canceled/)).toBeVisible();
  expect(libraryData).not.toHaveBeenCalled();
});
it('retains a clear conflict and requires a new preview after refused publication', async () => {
  panel();
  fireEvent.click(screen.getByRole('button', { name: 'Choose export folder & preview' }));
  await screen.findByRole('button', { name: 'Save reviewed export' });
  fireEvent.click(screen.getByRole('checkbox'));
  vi.mocked(libraryData).mockRejectedValueOnce(
    new Error('Destination changed; all versions retained.'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Save reviewed export' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Destination changed');
  expect(screen.queryByRole('button', { name: 'Save reviewed export' })).not.toBeInTheDocument();
});
it('offers cancellation while native work is pending without inventing a completed export', async () => {
  let reject: (error: Error) => void = () => {};
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((_, r) => {
        reject = r;
      }),
  );
  panel();
  fireEvent.click(screen.getByRole('button', { name: 'Choose export folder & preview' }));
  const cancel = await screen.findByRole('button', { name: 'Cancel export' });
  fireEvent.click(cancel);
  await waitFor(() => expect(library).toHaveBeenCalledWith({ kind: 'metadata_export_cancel' }));
  reject(new Error('Metadata export canceled. Partial stages retained.'));
  expect(await screen.findByRole('alert')).toHaveTextContent('canceled');
  expect(screen.queryByRole('button', { name: 'Save reviewed export' })).not.toBeInTheDocument();
});
it('exposes the export type as a labelled native select for keyboard and screen-reader users', () => {
  panel();
  expect(screen.getByRole('combobox', { name: 'Export type' })).toHaveValue('catalog');
});
