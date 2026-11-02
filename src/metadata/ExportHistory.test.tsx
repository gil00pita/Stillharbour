import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../test/render';
import { ExportHistory } from './ExportHistory';
import { metadata } from './ipc';
vi.mock('./ipc', () => ({ metadata: vi.fn() }));
it('keeps verified partial outputs incomplete until a completed receipt exists', async () => {
  vi.mocked(metadata).mockResolvedValue([
    { id: 'partial', lastPhase: 'file_verified', filesVerified: 1, updatedAt: '2026-11-02' },
    { id: 'complete', lastPhase: 'completed', filesVerified: 2, updatedAt: '2026-11-02' },
  ]);
  render(<ExportHistory />);
  screen.getByText('Export completion and retained partials').closest('details')!.open = true;
  fireEvent.click(screen.getByRole('button', { name: 'Load export history' }));
  expect((await screen.findByText('partial')).closest('li')).toHaveTextContent(
    'Incomplete — retained for review',
  );
  expect(screen.getByText('complete').closest('li')).toHaveTextContent('Completed');
  expect(metadata).toHaveBeenCalledWith({ kind: 'export_history' }, 'export_history');
});
it('shows typed history failures without claiming an empty or completed history', async () => {
  vi.mocked(metadata).mockRejectedValue(new Error('Catalog unavailable'));
  render(<ExportHistory />);
  screen.getByText('Export completion and retained partials').closest('details')!.open = true;
  fireEvent.click(screen.getByRole('button', { name: 'Load export history' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  expect(screen.queryByText('No export receipts yet.')).not.toBeInTheDocument();
});
