import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { restoreAssetFocus } from './focus';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  document.body.replaceChildren();
  vi.restoreAllMocks();
});
it('waits for a compact collection to remount before restoring its invoking photo', () => {
  restoreAssetFocus('generated', null);
  vi.advanceTimersByTime(125);
  const asset = document.createElement('article');
  asset.dataset.assetId = 'generated';
  const button = document.createElement('button');
  button.className = 'asset-open';
  asset.append(button);
  document.body.append(asset);
  const rects = Object.assign([new DOMRect(0, 0, 20, 20)], {
    item: () => new DOMRect(0, 0, 20, 20),
  });
  vi.spyOn(button, 'getClientRects').mockReturnValue(rects);
  vi.advanceTimersByTime(25);
  expect(button).toHaveFocus();
});
it('falls back to the collection after a bounded wait if the photo is no longer available', () => {
  const viewport = document.createElement('div');
  viewport.className = 'asset-viewport';
  viewport.tabIndex = 0;
  document.body.append(viewport);
  restoreAssetFocus('missing', null);
  vi.advanceTimersByTime(600);
  expect(viewport).toHaveFocus();
  expect(vi.getTimerCount()).toBe(0);
});
it('does not steal focus if the user moves elsewhere while the collection remounts', () => {
  const navigation = document.createElement('button');
  document.body.append(navigation);
  restoreAssetFocus('generated', null);
  navigation.focus();
  vi.advanceTimersByTime(600);
  expect(navigation).toHaveFocus();
  expect(vi.getTimerCount()).toBe(0);
});

it('restores the exact browsing offset after native dialog focus restoration', () => {
  const viewport = document.createElement('div');
  viewport.className = 'asset-viewport';
  const article = document.createElement('article');
  article.dataset.assetId = 'generated';
  const button = document.createElement('button');
  button.className = 'asset-open';
  article.append(button);
  viewport.append(article);
  document.body.append(viewport);
  vi.spyOn(button, 'getClientRects').mockReturnValue(
    Object.assign([new DOMRect(0, 0, 20, 20)], { item: () => new DOMRect(0, 0, 20, 20) }),
  );
  restoreAssetFocus('generated', button, 150);
  setTimeout(() => {
    button.focus();
    viewport.scrollTop = 36;
  }, 0); // Native dialog.close() finishes its focus scroll in a later task.
  vi.advanceTimersByTime(0);
  expect(viewport.scrollTop).toBe(36);
  vi.advanceTimersByTime(25);
  expect(viewport.scrollTop).toBe(150);
  expect(button).toHaveFocus();
});
