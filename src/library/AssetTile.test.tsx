import { fireEvent, render } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { ThumbnailResult } from '../bindings';
import { fixtureAsset } from './__tests__/fixtures';
import { TileImage } from './AssetTile';
vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc: () => 'thumbnail://localhost/' }));
const asset = fixtureAsset('generated-photo');
const result = (edge: number, state: string): ThumbnailResult => ({
  assetId: asset.id,
  revision: asset.revision,
  edge,
  state,
  error: state === 'unavailable' ? 'Decode budget reached' : null,
});
const props = { asset, imageHeight: 180, edge: 256, fit: 'contain' as const, offline: false };
it('keeps the tiny preview until the clear image actually loads, then reveals without changing geometry', () => {
  const { container, rerender } = render(
    <TileImage {...props} tinyPreview={result(32, 'ready')} preview={result(256, 'pending')} />,
  );
  expect(container.querySelectorAll('img')).toHaveLength(1);
  expect(container.querySelector('.photo-area')).toHaveStyle({ height: '180px' });
  expect(container.querySelector('.t-skel')).not.toHaveClass('is-revealed');
  rerender(
    <TileImage {...props} tinyPreview={result(32, 'ready')} preview={result(256, 'ready')} />,
  );
  expect(container.querySelectorAll('img')).toHaveLength(2);
  expect(container.querySelector('.t-skel')).not.toHaveClass('is-revealed');
  fireEvent.load(container.querySelector('img.t-skel-content')!);
  expect(container.querySelector('.t-skel')).toHaveClass('is-revealed');
  expect(container.querySelector('.photo-area')).toHaveStyle({ height: '180px' });
  fireEvent.error(container.querySelector('img.t-skel-content')!);
  expect(container.querySelector('.t-skel')).not.toHaveClass('is-revealed');
  expect(container.querySelectorAll('img')).toHaveLength(1);
});
it('retains a tiny cached image when clear decoding fails and keeps offline status explicit', () => {
  const { container, getByText } = render(
    <TileImage
      {...props}
      offline
      tinyPreview={result(32, 'ready')}
      preview={result(256, 'unavailable')}
    />,
  );
  expect(container.querySelector('img')).toHaveAttribute(
    'title',
    'Smaller preview · Decode budget reached',
  );
  expect(getByText('Offline original')).toBeVisible();
  fireEvent.error(container.querySelector('img')!);
  expect(getByText('Decode budget reached')).toBeVisible();
});

it('uses an indexed smaller clear rendition while a larger tier is loading or unavailable', () => {
  const { container, rerender } = render(
    <TileImage
      {...props}
      edge={1024}
      tinyPreview={result(32, 'ready')}
      preview={{ ...result(1024, 'pending'), fallbackEdge: 512 }}
    />,
  );
  expect(container.querySelector('img')).toHaveAttribute('src', expect.stringContaining('/512/'));
  fireEvent.error(container.querySelector('img')!);
  expect(container.querySelector('img')).toHaveAttribute('src', expect.stringContaining('/32/'));
  rerender(
    <TileImage
      {...props}
      edge={1024}
      tinyPreview={result(32, 'ready')}
      preview={{ ...result(1024, 'unavailable'), fallbackEdge: 512 }}
    />,
  );
  expect(container.querySelector('img')).toHaveAttribute('src', expect.stringContaining('/32/'));
});
