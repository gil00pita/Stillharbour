// Self-contained: WebDriver serializes this function into the native WebView.
export function warmPhotoForScroll() {
  const viewport = document.querySelector('.asset-viewport');
  const heading = document.querySelector('.date-heading');
  if (!viewport || !heading) return null;
  const bounds = viewport.getBoundingClientRect();
  if (Math.abs(heading.getBoundingClientRect().width - viewport.clientWidth) > 1) return null;
  for (const asset of document.querySelectorAll('.gallery .asset')) {
    const image = asset.querySelector('.t-skel-content');
    const area = asset.querySelector('.photo-area');
    if (!image?.complete || image.naturalWidth <= 32 || !area) continue;
    const frame = area.getBoundingClientRect();
    // Leave room for the sticky heading and the four-pixel test scroll.
    if (frame.top < bounds.top + 40 || frame.bottom > bounds.bottom - 8) continue;
    if (Math.abs(frame.width - (frame.height * image.naturalWidth) / image.naturalHeight) > 1)
      continue;
    return {
      id: asset.dataset.assetId,
      date: asset.dataset.date,
      source: image.getAttribute('src'),
      width: frame.width,
      height: frame.height,
      viewportWidth: viewport.clientWidth,
    };
  }
  return null;
}
