import { fireEvent, render, screen } from '../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { DateTimeline } from './DateTimeline';

beforeEach(() =>
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      disconnect() {}
    },
  ),
);
const buckets = [
  { year: '2026', count: 50, months: [{ period: '2026-03', count: 50 }] },
  { year: '1964', count: 2, months: [{ period: '1964-07', count: 2 }] },
  { year: 'Undated', count: 1, months: [{ period: 'Undated', count: 1 }] },
];
function setup(onIndex = vi.fn().mockReturnValue(false)) {
  const view = render(
    <DateTimeline
      buckets={buckets}
      date="2026-03-01"
      index={0}
      pageKey="first"
      scrolling={false}
      onYear={vi.fn()}
      onIndex={onIndex}
    />,
  );
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  vi.spyOn(slider, 'getBoundingClientRect').mockReturnValue({ top: 24, height: 352 } as DOMRect);
  return { ...view, slider, onIndex };
}
it('weights positions by photo count and follows the first visible photo', () => {
  const onYear = vi.fn();
  const { rerender } = render(
    <DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={onYear} />,
  );
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  expect(slider).toHaveAttribute('max', '52');
  rerender(<DateTimeline buckets={buckets} date="1964-07-01" scrolling onYear={onYear} />);
  expect(slider).toHaveValue('50');
  expect(slider).toHaveAttribute('aria-valuetext', 'Jul 1964');
  expect(screen.getByRole('navigation', { name: 'Photo timeline' })).toHaveClass('is-scrolling');
  fireEvent.click(screen.getByRole('button', { name: 'Jump to 2026, 50 photos' }));
  expect(onYear).toHaveBeenCalledWith('2026');
});
it('preserves an explicit Undated tail and the exact final photo', () => {
  render(
    <DateTimeline buckets={buckets} date="" progress={1} scrolling={false} onYear={vi.fn()} />,
  );
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  expect(slider).toHaveValue('52');
  expect(slider).toHaveAttribute('aria-valuetext', 'Undated');
});
it('navigates on pointer down and continuously while dragging before release', () => {
  const { slider, onIndex } = setup();
  fireEvent.pointerDown(slider, { pointerId: 1, clientY: 200 });
  expect(onIndex).toHaveBeenLastCalledWith(26);
  fireEvent.pointerMove(document, { pointerId: 1, clientY: 376 });
  expect(onIndex).toHaveBeenLastCalledWith(52);
  const calls = onIndex.mock.calls.length;
  fireEvent.pointerUp(document, { pointerId: 1, clientY: 376 });
  expect(onIndex).toHaveBeenCalledTimes(calls);
});
it('continues a captured drag outside the hit area and clamps at both ends', () => {
  const { slider, onIndex } = setup();
  fireEvent.pointerDown(slider, { pointerId: 2, clientY: 200 });
  fireEvent.pointerMove(document, { pointerId: 2, clientY: -100 });
  expect(onIndex).toHaveBeenLastCalledWith(0);
  fireEvent.pointerUp(document, { pointerId: 2, clientY: 1000 });
  expect(onIndex).toHaveBeenLastCalledWith(52);
});
it('keeps the latest requested position after release until the actual page arrives', () => {
  const { slider, rerender, onIndex } = setup();
  fireEvent.pointerDown(slider, { pointerId: 1, clientY: 376 });
  fireEvent.pointerUp(document, { pointerId: 1, clientY: 376 });
  expect(slider).toHaveValue('52');
  rerender(
    <DateTimeline
      buckets={buckets}
      date=""
      index={52}
      pageKey="last"
      scrolling={false}
      onYear={vi.fn()}
      onIndex={onIndex}
    />,
  );
  expect(slider).toHaveValue('52');
  rerender(
    <DateTimeline
      buckets={buckets}
      date="1964-07-01"
      index={50}
      pageKey="last"
      scrolling
      onYear={vi.fn()}
      onIndex={onIndex}
    />,
  );
  expect(slider).toHaveValue('50');
});
it('previews by the same photo scale and underline coordinate without seeking', () => {
  const { slider, onIndex, container } = setup();
  expect(screen.getByRole('status', { name: 'Currently visible date' })).toBeInTheDocument();
  fireEvent.pointerMove(slider, { clientY: 200 });
  expect(screen.getByLabelText('Timeline preview date')).toHaveTextContent('Mar 2026');
  expect(screen.getByLabelText('Timeline preview date')).toHaveStyle({ top: '50%' });
  expect(container.querySelector('.timeline-preview-marker')).toHaveStyle({ top: '50%' });
  expect(slider).toHaveValue('0');
  expect(onIndex).not.toHaveBeenCalled();
  fireEvent.pointerLeave(slider);
  expect(screen.getByLabelText('Currently visible date')).toHaveTextContent('Mar 2026');
});
it('supports live keyboard navigation, exact endpoints and a visible date label', () => {
  const { slider, onIndex } = setup();
  fireEvent.focus(slider);
  fireEvent.keyDown(slider, { key: 'ArrowDown' });
  expect(onIndex).toHaveBeenLastCalledWith(1);
  fireEvent.keyUp(slider, { key: 'ArrowDown' });
  fireEvent.keyDown(slider, { key: 'End' });
  expect(onIndex).toHaveBeenLastCalledWith(52);
  fireEvent.keyUp(slider, { key: 'End' });
  fireEvent.keyDown(slider, { key: 'Home' });
  expect(onIndex).toHaveBeenLastCalledWith(0);
  expect(slider).toHaveAttribute('aria-orientation', 'vertical');
});
it('advances separate arrow presses when several loaded photos share the first visible row', () => {
  const { slider, onIndex } = setup(vi.fn().mockReturnValue(true));
  fireEvent.focus(slider);
  for (let index = 1; index <= 4; index++) {
    fireEvent.keyDown(slider, { key: 'ArrowDown' });
    fireEvent.keyUp(slider, { key: 'ArrowDown' });
    expect(onIndex).toHaveBeenLastCalledWith(index);
    expect(slider).toHaveValue(String(index));
  }
});
it('retires a loaded-row keyboard target when the gallery scrolls or focus leaves', () => {
  const { slider, onIndex, rerender } = setup(vi.fn().mockReturnValue(true));
  fireEvent.keyDown(slider, { key: 'ArrowDown' });
  fireEvent.keyUp(slider, { key: 'ArrowDown' });
  expect(slider).toHaveValue('1');
  rerender(
    <DateTimeline
      buckets={buckets}
      date="2026-03-01"
      index={10}
      pageKey="first"
      scrolling
      onYear={vi.fn()}
      onIndex={onIndex}
    />,
  );
  expect(slider).toHaveValue('10');
  rerender(
    <DateTimeline
      buckets={buckets}
      date="2026-03-01"
      index={0}
      pageKey="first"
      scrolling
      onYear={vi.fn()}
      onIndex={onIndex}
    />,
  );
  expect(slider).toHaveValue('0');
  fireEvent.keyDown(slider, { key: 'ArrowDown' });
  fireEvent.keyUp(slider, { key: 'ArrowDown' });
  expect(onIndex).toHaveBeenLastCalledWith(1);
  expect(slider).toHaveValue('1');
  fireEvent.blur(slider);
  expect(slider).toHaveValue('0');
});
it.each(['pointercancel', 'lostpointercapture', 'Escape', 'blur'])(
  'stops %s without applying the subsequent release position',
  (kind) => {
    const { slider, onIndex } = setup();
    fireEvent.pointerDown(slider, { pointerId: 5, clientY: 200 });
    expect(onIndex).toHaveBeenCalledExactlyOnceWith(26);
    if (kind === 'Escape') fireEvent.keyDown(slider, { key: 'Escape' });
    else if (kind === 'blur') fireEvent.blur(slider);
    else fireEvent(document, new PointerEvent(kind, { bubbles: true, pointerId: 5 }));
    fireEvent.pointerUp(document, { pointerId: 5, clientY: 376 });
    expect(onIndex).toHaveBeenCalledExactlyOnceWith(26);
  },
);
it('scrolls throughout a single dense month and disables only an empty/single-photo rail', () => {
  const onPosition = vi.fn().mockReturnValue(true);
  const { rerender } = render(
    <DateTimeline
      buckets={[buckets[0]!]}
      date="2026-03-01"
      progress={0.6}
      scrolling={false}
      onYear={vi.fn()}
      onPosition={onPosition}
    />,
  );
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  expect(slider).not.toBeDisabled();
  expect(Number((slider as HTMLInputElement).value)).toBeCloseTo(29.4);
  fireEvent.change(slider, { target: { value: '40' } });
  expect(onPosition).toHaveBeenCalledWith('2026-03', 40 / 49);
  rerender(<DateTimeline buckets={[]} date="" scrolling={false} onYear={vi.fn()} />);
  expect(slider).toBeDisabled();
  expect(slider).toHaveValue('0');
});
it('forwards wheel input without intercepting Ctrl+wheel or duplicating default scrolling', () => {
  const onScroll = vi.fn();
  render(
    <DateTimeline
      buckets={buckets}
      date="2026-03-01"
      scrolling={false}
      onYear={vi.fn()}
      onScroll={onScroll}
    />,
  );
  const rail = screen.getByRole('navigation', { name: 'Photo timeline' });
  const event = new WheelEvent('wheel', {
    deltaY: 120,
    deltaMode: 0,
    bubbles: true,
    cancelable: true,
  });
  rail.dispatchEvent(event);
  expect(onScroll).toHaveBeenCalledWith(120, 0);
  expect(event.defaultPrevented).toBe(true);
  fireEvent.wheel(rail, { deltaY: 50, ctrlKey: true });
  expect(onScroll).toHaveBeenCalledOnce();
});
