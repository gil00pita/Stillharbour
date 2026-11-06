import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import { KeywordImport } from '@/components/KeywordImport';
import { metadata } from '../../metadata/ipc';
import { libraryData } from '../../ipc';
vi.mock('../../metadata/ipc', () => ({ metadata: vi.fn() }));
vi.mock('../../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(metadata)
    .mockReset()
    .mockResolvedValue({} as never);
  vi.mocked(libraryData)
    .mockReset()
    .mockResolvedValue({ revision: '17' } as never);
});
function mount(onImported = vi.fn()) {
  render(<KeywordImport assetId="generated-asset" onImported={onImported} />);
  screen.getByText('Import observed keywords').closest('details')!.open = true;
  return onImported;
}
it('imports only after explicit action and restores removed labels only after an explicit check', async () => {
  const imported = mount();
  expect(metadata).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Import keywords into catalog' }));
  await waitFor(() => expect(imported).toHaveBeenCalledTimes(1));
  expect(metadata).toHaveBeenLastCalledWith(
    {
      kind: 'import_keywords',
      asset_id: 'generated-asset',
      expected_revision: '17',
      restore_removed: false,
    },
    'notes',
  );
  fireEvent.click(screen.getByLabelText('Restore previously removed observed labels too'));
  fireEvent.click(screen.getByRole('button', { name: 'Import keywords into catalog' }));
  await waitFor(() => expect(imported).toHaveBeenCalledTimes(2));
  expect(metadata).toHaveBeenLastCalledWith(
    {
      kind: 'import_keywords',
      asset_id: 'generated-asset',
      expected_revision: '17',
      restore_removed: true,
    },
    'notes',
  );
});
it('clears previous success when a later import fails or has no retained keywords', async () => {
  const imported = mount();
  fireEvent.click(screen.getByRole('button', { name: 'Import keywords into catalog' }));
  await waitFor(() => expect(imported).toHaveBeenCalledTimes(1));
  expect(screen.getByRole('status')).toHaveTextContent('saved in the catalog');
  vi.mocked(metadata).mockRejectedValueOnce(
    new Error('No retained source keywords are available to import.'),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Import keywords into catalog' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('No retained source keywords');
  expect(screen.getByRole('status')).toBeEmptyDOMElement();
  expect(imported).toHaveBeenCalledTimes(1);
});
