import { FavoriteHeart } from './FavoriteHeart';
import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { fixtureAsset } from './__tests__/fixtures';
import { OriginalPhoto } from './OriginalPhoto';
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (_path: string, scheme: string) => `${scheme}://cache/`,
}));
it('loads original bytes by ID, keeps the preview until actual load, and provides honest retry on failure', () => {
  const asset = fixtureAsset('original');
  const onReady = vi.fn();
  const { container } = render(
    <OriginalPhoto
      asset={asset}
      results={[2048, 512, 32].map((edge) => ({
        assetId: asset.id,
        revision: asset.revision,
        edge,
        state: 'ready',
        error: null,
      }))}
      initialSrc={undefined}
      transportError={null}
      onReady={onReady}
      onRetry={vi.fn()}
    />,
  );
  const original = container.querySelector<HTMLImageElement>('.original-image')!;
  expect(original.src).toContain(`original://cache/${asset.id}/${asset.revision}/`);
  expect(screen.getByText('Loading original…')).toBeVisible();
  fireEvent.load(original);
  expect(container.querySelector('.original-photo')).toHaveClass('has-original');
  expect(container.querySelector('.original-fallback')).toHaveAttribute('aria-hidden', 'true');
  expect(onReady).toHaveBeenLastCalledWith(true);
  expect(screen.queryByText('Loading original…')).not.toBeInTheDocument();
  fireEvent.error(original);
  fireEvent.error(original);
  expect(screen.getByText(/Original unavailable/)).toBeVisible();
  expect(screen.getByText(/Original unavailable/)).toHaveAttribute(
    'title',
    expect.stringContaining('HEIC/HEIF on compatible macOS webviews'),
  );
  expect(onReady).toHaveBeenLastCalledWith(false);
  fireEvent.click(screen.getByRole('button', { name: 'Retry original' }));
  expect(container.querySelector('.original-image')).toHaveAttribute(
    'src',
    expect.stringContaining('request=1'),
  );
});

it('snaps confirmed heart feedback when native window visibility suppresses motion', () => {
  const hidden = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  const { container } = render(
    <FavoriteHeart favorite busy={false} animate onClick={vi.fn()} onAnimationEnd={vi.fn()} />,
  );
  expect(container.querySelector('.viewer-favorite')).toHaveClass('without-motion');
  hidden.mockRestore();
});
