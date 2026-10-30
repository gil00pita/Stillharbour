import { useLayoutEffect, useRef, type RefObject } from 'react';
import type { AssetSummary } from '../bindings';

export type PhotoOrigin = { rect: DOMRect; src: string };
export function photoOrigin(opener: HTMLElement | null): PhotoOrigin | undefined {
  if (!opener?.isConnected) return;
  const images = [...opener.querySelectorAll<HTMLImageElement>('img')].filter(
    (image) => image.complete && image.naturalWidth > 0 && getComputedStyle(image).opacity !== '0',
  );
  const image = images.sort((a, b) => b.naturalWidth - a.naturalWidth)[0];
  if (!image) return;
  const rect = image.getBoundingClientRect();
  const viewport = opener.closest('.asset-viewport')?.getBoundingClientRect();
  if (viewport && (rect.bottom <= viewport.top || rect.top >= viewport.bottom)) return;
  if (!rect.width || !rect.height || rect.bottom <= 0 || rect.top >= innerHeight) return;
  // Contained images can have letterboxing inside a tile's image element.
  if (getComputedStyle(image).objectFit === 'contain') {
    const scale = Math.min(rect.width / image.naturalWidth, rect.height / image.naturalHeight);
    const width = image.naturalWidth * scale;
    const height = image.naturalHeight * scale;
    return {
      src: image.currentSrc || image.src,
      rect: new DOMRect(
        rect.x + (rect.width - width) / 2,
        rect.y + (rect.height - height) / 2,
        width,
        height,
      ),
    };
  }
  return { src: image.currentSrc || image.src, rect };
}
function tileOrigin(id: string) {
  const tile = [...document.querySelectorAll<HTMLElement>('[data-asset-id]')].find(
    (element) => element.dataset.assetId === id,
  );
  return photoOrigin(tile ?? null);
}
function reducedMotion() {
  return (
    document.visibilityState === 'hidden' ||
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  );
}
function motion(
  frame: HTMLElement,
  rect: DOMRect | undefined,
  opening: boolean,
  current?: Keyframe,
) {
  if (!frame.animate || reducedMotion()) return null;
  const box = frame.getBoundingClientRect();
  const transform =
    rect && box.width && box.height
      ? `translate(${rect.left - box.left}px, ${rect.top - box.top}px) scale(${rect.width / box.width}, ${rect.height / box.height})`
      : 'scale(0.96)';
  const styles = getComputedStyle(frame);
  const duration =
    parseFloat(styles.getPropertyValue(opening ? '--viewer-open-ms' : '--viewer-close-ms')) ||
    (opening ? 250 : 150);
  const start = { transform, opacity: rect ? 1 : 0 };
  const end = { transform: 'none', opacity: 1 };
  return frame.animate(opening ? [start, end] : [current ?? end, start], {
    duration,
    easing: styles.getPropertyValue('--viewer-ease').trim() || 'cubic-bezier(0.22, 1, 0.36, 1)',
    fill: 'both',
  });
}

export function usePhotoMotion(
  asset: AssetSummary,
  origin: PhotoOrigin | undefined,
  onClose: () => void,
  closeHandleRef: RefObject<(() => void) | null>,
) {
  const frame = useRef<HTMLDivElement>(null);
  const callback = useRef(onClose);
  useLayoutEffect(() => {
    callback.current = onClose;
  }, [onClose]);
  useLayoutEffect(() => {
    const element = frame.current;
    const plane = element?.parentElement;
    if (!element || !plane) return;
    let animation: Animation | null = null;
    let disposed = false;
    let closing = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    function fit() {
      const ratio =
        asset.width && asset.height
          ? asset.width / asset.height
          : origin
            ? origin.rect.width / origin.rect.height
            : 1;
      const width = Math.min(plane!.clientWidth, plane!.clientHeight * ratio);
      element!.style.width = `${width}px`;
      element!.style.height = `${width / ratio}px`;
    }
    fit();
    const observer = new ResizeObserver(fit);
    observer.observe(plane);
    const viewer = element.closest<HTMLElement>('.photo-viewer');
    function finish() {
      if (disposed) return;
      clearTimeout(timeout);
      animation?.cancel();
      viewer?.setAttribute('data-motion', 'open');
    }
    if (origin) {
      viewer?.setAttribute('data-motion', 'opening');
      animation = motion(element, origin.rect, true);
      if (animation) {
        void animation.finished.then(finish, () => {});
        timeout = setTimeout(
          finish,
          Number(animation.effect?.getComputedTiming().duration || 250) + 100,
        );
      } else finish();
    } else finish();
    closeHandleRef.current = () => {
      if (closing) return;
      closing = true;
      clearTimeout(timeout);
      const painted = getComputedStyle(element);
      const current = { transform: painted.transform, opacity: painted.opacity };
      animation?.cancel();
      viewer?.setAttribute('data-motion', 'closing');
      const destination = tileOrigin(asset.id);
      animation = motion(element, destination?.rect, false, current);
      let finished = false;
      const closed = () => {
        if (!disposed && !finished) {
          finished = true;
          clearTimeout(timeout);
          callback.current();
        }
      };
      if (animation) {
        void animation.finished.then(closed, () => {});
        // Native occlusion may suspend an animation's completion notification.
        timeout = setTimeout(
          closed,
          Number(animation.effect?.getComputedTiming().duration || 150) + 70,
        );
      } else closed();
    };
    return () => {
      disposed = true;
      closeHandleRef.current = null;
      clearTimeout(timeout);
      animation?.cancel();
      observer.disconnect();
    };
  }, [asset.id, asset.width, asset.height, origin, closeHandleRef]);
  return frame;
}
