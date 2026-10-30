import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { ThumbnailResult } from '../bindings';
import { ViewerImage } from './ViewerImage';
import { fixtureAsset } from './__tests__/fixtures';
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: () => 'thumbnail://cache/' }));
const asset = fixtureAsset('viewer');
function result(edge: number, state = 'ready'): ThumbnailResult {
  return {
    assetId: asset.id,
    revision: asset.revision,
    edge,
    state,
    error: state === 'unavailable' ? 'High detail unavailable' : null,
  };
}
it('retains the small layer until actual high-detail load and on high-detail image failure', () => {
  const { container } = render(
    <ViewerImage
      asset={asset}
      results={[result(2048), result(512), result(32)]}
      transportError={null}
      onRetry={vi.fn()}
    />,
  );
  const layers = container.querySelector('.viewer-renditions')!;
  const high = container.querySelector('.viewer-rendition-high')!;
  fireEvent.load(container.querySelector('.viewer-rendition-small')!);
  expect(layers).not.toHaveClass('is-revealed');
  expect(screen.getByRole('img', { name: asset.name })).toHaveAttribute(
    'src',
    expect.stringContaining('/512/'),
  );
  fireEvent.load(high);
  expect(layers).toHaveClass('is-revealed');
  fireEvent.error(high);
  fireEvent.error(high);
  expect(layers).not.toHaveClass('is-revealed');
  expect(screen.getByRole('img', { name: asset.name })).toHaveAttribute(
    'src',
    expect.stringContaining('/512/'),
  );
  expect(screen.getByRole('status')).toHaveTextContent('Smaller cached preview');
});
it('shows an existing tiny cache image when clearer tiers are unavailable offline', () => {
  render(
    <ViewerImage
      asset={asset}
      results={[result(2048, 'unavailable'), result(512, 'unavailable'), result(32)]}
      transportError={null}
      onRetry={vi.fn()}
    />,
  );
  expect(screen.getByRole('img', { name: asset.name })).toHaveAttribute(
    'src',
    expect.stringContaining('/32/'),
  );
  fireEvent.load(screen.getByRole('img', { name: asset.name }));
  expect(screen.getByRole('status')).toHaveTextContent('High detail unavailable');
});

it('reuses a cached1024px gallery image when the requested viewer tiers are unavailable offline', () => {
  render(
    <ViewerImage
      asset={asset}
      results={[
        { ...result(2048, 'unavailable'), fallbackEdge: 1024 },
        result(512, 'unavailable'),
        result(32, 'unavailable'),
      ]}
      transportError={null}
      onRetry={vi.fn()}
    />,
  );
  expect(screen.getByRole('img', { name: asset.name })).toHaveAttribute(
    'src',
    expect.stringContaining('/1024/'),
  );
});

it('shows glare only while awaiting actual image load, then stops on ready or terminal failure', () => {
  const { container } = render(
    <ViewerImage asset={asset} results={[result(2048)]} transportError={null} onRetry={vi.fn()} />,
  );
  expect(container.querySelector('.preview-loading-sheen')).not.toBeNull();
  const image = container.querySelector('img')!;
  fireEvent.load(image);
  expect(container.querySelector('.preview-loading-sheen')).toBeNull();
  expect(container.querySelector('.viewer-renditions')).toHaveAttribute('aria-busy', 'false');
  fireEvent.error(image);
  fireEvent.error(image);
  expect(container.querySelector('.preview-loading-sheen')).toBeNull();
  expect(screen.getByRole('button', { name: 'Retry preview' })).toBeEnabled();
});
