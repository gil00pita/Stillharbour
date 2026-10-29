import { afterEach, expect, it, vi } from 'vitest';
import { warmPhotoForScroll } from './warmPhoto.mjs';

afterEach(() => {
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});
function fixture() {
  document.body.innerHTML =
    '<div class="asset-viewport"><div class="date-heading"></div><div class="gallery"></div></div>';
  const viewport = document.querySelector('.asset-viewport');
  Object.defineProperty(viewport, 'clientWidth', { value: 800 });
  vi.spyOn(viewport, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 100, 800, 600));
  const heading = document.querySelector('.date-heading');
  vi.spyOn(heading, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 100, 800, 36));
}
function photo(id, top, complete = true, width = 300) {
  const article = document.createElement('article');
  article.className = 'asset';
  article.dataset.assetId = id;
  article.dataset.date = '2026-10-29';
  article.innerHTML =
    '<span class="photo-area"><img class="t-skel-content" src="thumbnail://fixture/512"></span>';
  const image = article.querySelector('img');
  Object.defineProperties(image, {
    complete: { value: complete },
    naturalWidth: { value: complete ? 600 : 0 },
    naturalHeight: { value: complete ? 400 : 0 },
  });
  vi.spyOn(article.querySelector('.photo-area'), 'getBoundingClientRect').mockReturnValue(
    new DOMRect(0, top, width, 200),
  );
  document.querySelector('.gallery').append(article);
  return image;
}
it('selects a loaded visible photo instead of an overscan node or unfinished image', () => {
  fixture();
  photo('overscan', -200);
  photo('loading', 200, false);
  photo('visible', 420);
  expect(warmPhotoForScroll()?.id).toBe('visible');
});
it('waits while decoded pixels and the tile frame disagree', () => {
  fixture();
  photo('stale-layout', 200, true, 200);
  expect(warmPhotoForScroll()).toBeNull();
});
it('waits for viewport resize geometry and a completely loaded photo', () => {
  fixture();
  const image = photo('pending', 200, false);
  expect(warmPhotoForScroll()).toBeNull();
  image.closest('.asset').remove();
  photo('loaded', 200);
  const heading = document.querySelector('.date-heading');
  vi.mocked(heading.getBoundingClientRect).mockReturnValue(new DOMRect(0, 100, 700, 36));
  expect(warmPhotoForScroll()).toBeNull();
});
