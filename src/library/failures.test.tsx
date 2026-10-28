import { fixturePreferences } from './__tests__/fixtures';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import { Inspector } from './Inspector';
import { LibraryScreen } from './LibraryScreen';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
});
describe('native read failure states', () => {
  it('shows query failure without claiming an empty or current result set and retries', async () => {
    vi.mocked(libraryData)
      .mockRejectedValueOnce(new Error('Catalog is busy.'))
      .mockResolvedValueOnce([]);
    const onError = vi.fn();
    render(
      <LibraryScreen
        defaults={fixturePreferences()}
        status={{
          roots: [],
          jobs: [],
          revision: '1',
          previewGeneration: '0',
          metadataAvailable: true,
          previewAvailable: true,
        }}
        section="Photos"
        onError={onError}
        onRefresh={vi.fn()}
        onSettings={vi.fn()}
        onRelink={vi.fn()}
      />,
    );
    expect(
      await screen.findByRole('heading', { name: 'Library couldn’t be loaded' }),
    ).toBeVisible();
    expect(screen.queryByText('No matching photos yet')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Select all results' })).not.toBeInTheDocument();
    vi.mocked(libraryData).mockResolvedValueOnce({
      assets: [],
      total: 0,
      nextCursor: null,
      revision: '1',
    });
    fireEvent.click(screen.getByRole('button', { name: 'Retry library' }));
    expect(await screen.findByText('No matching photos yet')).toBeVisible();
    expect(onError).toHaveBeenCalled();
  });
  it('gives Inspector a failed state, retry and initial keyboard focus', async () => {
    vi.mocked(libraryData).mockRejectedValue(new Error('Photo is unavailable.'));
    render(
      <Inspector
        id="generated"
        revision="1"
        onError={vi.fn()}
        onClose={vi.fn()}
        onAnnotate={vi.fn()}
      />,
    );
    expect(await screen.findByText('Photo details couldn’t be loaded.')).toBeVisible();
    expect(screen.queryByText('Loading details…')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Photo details' })).toHaveFocus();
    fireEvent.click(screen.getByRole('button', { name: 'Retry details' }));
    await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(2));
  });
});
