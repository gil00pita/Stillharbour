import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { expect, it, vi } from 'vitest';
import type { ThumbnailResult } from '../../bindings';
import { fixtureAsset } from '../../library/__tests__/fixtures';
import { AssetTile, TileImage } from '@/components/AssetTile';
import { PhotoAlbumActions } from '@/components/PhotoAlbumActions';

it('provides hover and keyboard selection in the actual inactive selection context and respects its busy guard', () => {
  const choose = vi.fn();
  const onSelect = vi.fn();
  const asset = fixtureAsset('hover');
  const status = {
    roots: [],
    jobs: [],
    revision: '0',
    previewGeneration: '0',
    metadataAvailable: true,
    previewAvailable: true,
  };
  const tile = (
    <AssetTile
      asset={asset}
      mode="gallery"
      width={240}
      imageHeight={160}
      selected={false}
      selectionActive={false}
      frozen={false}
      offline={false}
      edge={256}
      fit="contain"
      preview={undefined}
      columns={[]}
      onSelect={onSelect}
      onOpen={vi.fn()}
      onNavigate={vi.fn()}
    />
  );
  const view = render(
    <PhotoAlbumActions
      status={status}
      onChanged={async () => {}}
      selection={{ active: false, disabled: false, onSelect: choose }}
    >
      {tile}
    </PhotoAlbumActions>,
  );
  fireEvent.click(screen.getByRole('checkbox', { name: 'Select Generated hover.jpg' }));
  expect(choose).toHaveBeenCalledWith(asset);
  expect(onSelect).not.toHaveBeenCalled();
  view.rerender(
    <PhotoAlbumActions
      status={status}
      onChanged={async () => {}}
      selection={{ active: false, disabled: true, onSelect: choose }}
    >
      {tile}
    </PhotoAlbumActions>,
  );
  expect(screen.getByRole('checkbox', { name: 'Select Generated hover.jpg' })).toBeDisabled();
});

it('saves a thumbnail favorite through its asset ID and reports a failed save without opening the photo', async () => {
  const onFavorite = vi.fn().mockRejectedValueOnce(new Error('Catalog unavailable'));
  const onOpen = vi.fn();
  const props = {
    asset: fixtureAsset('favorite'),
    mode: 'gallery' as const,
    width: 240,
    imageHeight: 160,
    selected: false,
    selectionActive: false,
    frozen: false,
    offline: false,
    edge: 256,
    fit: 'contain' as const,
    preview: undefined,
    columns: [],
    onSelect: vi.fn(),
    onOpen,
    onNavigate: vi.fn(),
    onFavorite,
  };
  const view = render(<AssetTile {...props} />);
  fireEvent.click(screen.getByRole('button', { name: 'Favorite Generated favorite.jpg' }));
  expect(onFavorite).toHaveBeenCalledWith('favorite', true);
  expect(await screen.findByRole('alert')).toHaveTextContent('Favorite couldn’t be saved');
  expect(onOpen).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Favorite Generated favorite.jpg' })).toHaveAttribute(
    'aria-pressed',
    'false',
  );
  onFavorite.mockResolvedValueOnce(undefined);
  await act(async () =>
    fireEvent.click(screen.getByRole('button', { name: 'Favorite Generated favorite.jpg' })),
  );
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  view.rerender(<AssetTile {...props} asset={{ ...props.asset, favorite: true }} favoriteBusy />);
  expect(screen.getByRole('button', { name: 'Unfavorite Generated favorite.jpg' })).toBeDisabled();
});
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
  expect(container.querySelector('img')).toHaveClass('photo-tiny-preview');
  expect(container.querySelector('.photo-area')).toHaveStyle({ height: '180px' });
  expect(container.querySelector('.t-skel')).not.toHaveClass('is-revealed');
  rerender(
    <TileImage {...props} tinyPreview={result(32, 'ready')} preview={result(256, 'ready')} />,
  );
  expect(container.querySelectorAll('img')).toHaveLength(2);
  expect(container.querySelector('.t-skel-content')).not.toHaveClass('photo-tiny-preview');
  expect(container.querySelector('.t-skel')).not.toHaveClass('is-revealed');
  fireEvent.load(container.querySelector('img.t-skel-content')!);
  expect(container.querySelector('.t-skel')).toHaveClass('is-revealed');
  expect(container.querySelector('.photo-area')).toHaveStyle({ height: '180px' });
  fireEvent.error(container.querySelector('img.t-skel-content')!);
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
  expect(container.querySelector('img')).not.toHaveClass('photo-tiny-preview');
  fireEvent.error(container.querySelector('img')!);
  fireEvent.error(container.querySelector('img')!);
  expect(container.querySelector('img')).toHaveAttribute('src', expect.stringContaining('/32/'));
  expect(container.querySelector('img')).toHaveClass('photo-tiny-preview');
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

it('reveals immediately when an occluded native window suspends transitions', () => {
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  const { container } = render(<TileImage {...props} preview={result(256, 'ready')} />);
  fireEvent.load(container.querySelector('.t-skel-content')!);
  expect(container.querySelector('.photo-layers')).toHaveClass('without-motion', 'is-revealed');
  vi.restoreAllMocks();
});

it('retains an already-loaded clear image while scroll-generation status refreshes', () => {
  const { container, rerender } = render(
    <TileImage {...props} tinyPreview={result(32, 'ready')} preview={result(256, 'ready')} />,
  );
  const clear = container.querySelector('.t-skel-content')!;
  fireEvent.load(clear);
  rerender(<TileImage {...props} tinyPreview={result(32, 'ready')} preview={undefined} />);
  expect(container.querySelector('.t-skel-content')).toBe(clear);
  expect(container.querySelector('.t-skel')).toHaveClass('is-revealed');
});

it('recovers a transient cached-image transport failure without a library rescan', async () => {
  const { container, queryByText } = render(
    <TileImage {...props} preview={result(256, 'ready')} />,
  );
  fireEvent.error(container.querySelector('.t-skel-content')!);
  // A ready cache response must get a bounded fresh image request instead of
  // latching the failure until this virtual tile happens to unmount.
  const retried = container.querySelector<HTMLImageElement>('.t-skel-content');
  expect(retried).not.toBeNull();
  expect(retried?.src).toContain('retry=1');
  fireEvent.load(retried!);
  expect(queryByText('Cached preview failed to load')).not.toBeInTheDocument();
});
