import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ComparisonPreviews } from './ComparisonPreviews';
import { libraryData } from '../ipc';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
it('loads each side by row identity and keeps decoder failures as text without a fidelity claim', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind !== 'backup_comparison_preview') throw new Error('Unexpected request');
    if (request.side === 'destination') throw new Error('<script>Unsupported image</script>');
    return {
      runId: 'run',
      entryId: 'entry',
      side: 'source',
      png: [137, 80, 78, 71],
      message: 'Preview is not integrity or recovery evidence.',
    };
  });
  render(<ComparisonPreviews runId="run" entryId="entry" />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Load side-by-side previews' }));
  expect(await screen.findByAltText('source comparison preview')).toHaveAttribute(
    'src',
    'data:image/png;base64,iVBORw==',
  );
  expect(await screen.findByText('<script>Unsupported image</script>')).toBeVisible();
  expect(document.querySelector('script')).toBeNull();
  await waitFor(() => expect(screen.getByRole('button')).toBeEnabled());
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'backup_comparison_preview', run_id: 'run', entry_id: 'entry', side: 'destination' },
    'backup_comparison_preview',
  );
});
