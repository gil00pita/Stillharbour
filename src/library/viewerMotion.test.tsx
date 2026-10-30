import { act, render } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { fixtureAsset } from './__tests__/fixtures';
import { usePhotoMotion } from './viewerMotion';

const origin = { rect: new DOMRect(40, 160, 120, 80), src: 'thumbnail://fixture/32/policy' };
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  Reflect.deleteProperty(HTMLElement.prototype, 'animate');
});
function scene(reduce = false) {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  );
  vi.stubGlobal('matchMedia', () => ({ matches: reduce }));
  vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(600);
  vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(400);
  vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockReturnValue(
    new DOMRect(300, 100, 600, 400),
  );
  const animations: { cancel: ReturnType<typeof vi.fn>; finish: () => void; frames: Keyframe[] }[] =
    [];
  Object.defineProperty(HTMLElement.prototype, 'animate', {
    configurable: true,
    value: (frames: Keyframe[]) => {
      let finish = () => {};
      const finished = new Promise<void>((resolve) => {
        finish = resolve;
      });
      const animation = {
        cancel: vi.fn(),
        finished,
        effect: { getComputedTiming: () => ({ duration: 150 }) },
      };
      animations.push({ cancel: animation.cancel, finish, frames });
      return animation;
    },
  });
  return animations;
}
function Harness({
  closeRef,
  onClose,
}: {
  closeRef: { current: (() => void) | null };
  onClose: () => void;
}) {
  const frame = usePhotoMotion(fixtureAsset('motion'), origin, onClose, closeRef);
  return (
    <div className="photo-viewer">
      <div>
        <div ref={frame} />
      </div>
    </div>
  );
}
it('zooms from the thumbnail, keeps the close transition mounted, and completes close once', async () => {
  const animations = scene();
  const close = { current: null as (() => void) | null };
  const onClose = vi.fn();
  render(<Harness closeRef={close} onClose={onClose} />);
  expect(animations[0]?.frames[0]?.transform).toContain('translate(-260px, 60px)');
  await act(async () => {
    animations[0]?.finish();
  });
  act(() => close.current?.());
  act(() => close.current?.());
  expect(animations).toHaveLength(2);
  expect(onClose).not.toHaveBeenCalled();
  await act(async () => {
    animations[1]?.finish();
  });
  expect(onClose).toHaveBeenCalledOnce();
});
it('skips spatial animation for reduced motion and cancels opening work on navigation away', () => {
  const animations = scene(true);
  const close = { current: null as (() => void) | null };
  const onClose = vi.fn();
  const rendered = render(<Harness closeRef={close} onClose={onClose} />);
  act(() => close.current?.());
  expect(animations).toHaveLength(0);
  expect(onClose).toHaveBeenCalledOnce();
  rendered.unmount();
  expect(close.current).toBeNull();
});
