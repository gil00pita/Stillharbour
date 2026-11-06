import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import { DuplicateComparison } from '@/components/DuplicateComparison';
import { fixtureAsset } from '../../library/__tests__/fixtures';
import { libraryData } from '../../ipc';
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: () => 'thumbnail://cache/' }));
const assets = [fixtureAsset('a'), fixtureAsset('b')];
beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'thumbnails')
        return request.keys.map((key) => ({ ...key, state: 'ready', error: null })) as never;
      throw new Error('Unexpected request');
    });
});
it('switches all three modes and changes the clip position without treating previews as equality evidence', async () => {
  render(<DuplicateComparison assets={assets} />);
  await screen.findByRole('img', { name: assets[0]!.name });
  fireEvent.click(screen.getByRole('button', { name: 'Slider' }));
  const slider = screen.getByRole('slider', { name: 'Comparison slider' });
  await waitFor(() => expect(slider).toBeEnabled());
  fireEvent.change(slider, { target: { value: '72' } });
  expect(document.querySelectorAll('figure')[1]).toHaveStyle({ clipPath: 'inset(0 0 0 72%)' });
  fireEvent.click(screen.getByRole('button', { name: 'Difference heatmap' }));
  expect(document.querySelectorAll('.duplicate-heat-layer')).toHaveLength(2);
  expect(screen.getByText(/Previews are unaligned/)).toBeVisible();
  expect(screen.getByText(/visual inspection cannot prove byte equality/)).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Side by side' }));
  expect(document.querySelector('.duplicate-stage')).toHaveClass('duplicate-stage-pair');
});
it('disables the overlay slider when either preview fails and offers retry', async () => {
  render(<DuplicateComparison assets={assets} />);
  fireEvent.error(await screen.findByRole('img', { name: assets[0]!.name }));
  fireEvent.click(screen.getByRole('button', { name: 'Slider' }));
  expect(screen.getByRole('slider', { name: 'Comparison slider' })).toBeDisabled();
  expect(screen.getByText('Preview failed to load.')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Retry comparison previews' }));
  expect(await screen.findByRole('img', { name: assets[0]!.name })).toBeVisible();
});
