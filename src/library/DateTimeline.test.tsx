import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
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
it('follows the visible year and month while keeping year navigation keyboard accessible', () => {
  const onYear = vi.fn();
  const { rerender } = render(
    <DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={onYear} />,
  );
  const rail = screen.getByRole('navigation', { name: 'Photo timeline' });
  expect(rail).not.toHaveClass('is-scrolling');
  expect(screen.getByRole('button', { name: 'Jump to 2026, 50 photos' })).toHaveAttribute(
    'aria-current',
    'date',
  );
  rerender(<DateTimeline buckets={buckets} date="1964-07-01" scrolling onYear={onYear} />);
  expect(rail).toHaveClass('is-scrolling');
  expect(screen.getByLabelText('Currently visible date')).toHaveTextContent('1964');
  expect(screen.getByLabelText('Currently visible date')).toHaveAttribute('aria-live', 'off');
  expect(screen.getByRole('button', { name: 'Jump to 1964, 2 photos' })).toHaveAttribute(
    'aria-current',
    'date',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Jump to 2026, 50 photos' }));
  expect(onYear).toHaveBeenCalledWith('2026');
});
describe('unknown dates', () => {
  it('keeps Undated separate and navigable', () => {
    const onYear = vi.fn();
    render(<DateTimeline buckets={buckets} date="" scrolling onYear={onYear} />);
    expect(screen.getByLabelText('Currently visible date')).toHaveTextContent('Undated');
    fireEvent.click(screen.getByRole('button', { name: 'Jump to Undated, 1 photos' }));
    expect(onYear).toHaveBeenCalledWith('Undated');
  });
});

it('commits a real month seek on scrubber keyboard release rather than loading intervening photos', () => {
  const onYear = vi.fn();
  render(<DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={onYear} />);
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  fireEvent.focus(slider);
  fireEvent.change(slider, { target: { value: '1' } });
  expect(onYear).not.toHaveBeenCalled();
  fireEvent.keyUp(slider, { key: 'ArrowDown' });
  expect(onYear).toHaveBeenCalledWith('1964-07');
});

it('follows gallery scrolling after the scrubber has received focus', () => {
  const { rerender } = render(
    <DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={vi.fn()} />,
  );
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  fireEvent.focus(slider);
  rerender(<DateTimeline buckets={buckets} date="1964-07-01" scrolling onYear={vi.fn()} />);
  expect(slider).toHaveValue('1');
  expect(slider).toHaveAttribute('aria-valuetext', 'Jul 1964');
});
it('commits a drag released outside the original hit area', () => {
  const onYear = vi.fn();
  render(<DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={onYear} />);
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  fireEvent.pointerDown(slider, { pointerId: 1 });
  fireEvent.change(slider, { target: { value: '1' } });
  fireEvent.pointerUp(document, { pointerId: 1 });
  expect(onYear).toHaveBeenLastCalledWith('1964-07');
});

it('forwards wheel input on the rail without zoom interception or duplicate default scrolling', () => {
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
  const wheel = new WheelEvent('wheel', {
    deltaY: 120,
    deltaMode: 0,
    bubbles: true,
    cancelable: true,
  });
  rail.dispatchEvent(wheel);
  expect(onScroll).toHaveBeenCalledWith(120, 0);
  expect(wheel.defaultPrevented).toBe(true);
  fireEvent.wheel(rail, { deltaY: 50, ctrlKey: true });
  expect(onScroll).toHaveBeenCalledTimes(1);
});
it.each(['pointercancel', 'lostpointercapture'])('cancels %s without seeking', (type) => {
  const onYear = vi.fn();
  render(<DateTimeline buckets={buckets} date="2026-03-01" scrolling={false} onYear={onYear} />);
  const slider = screen.getByRole('slider', { name: 'Jump to year or month' });
  fireEvent.pointerDown(slider, { pointerId: 1 });
  fireEvent.change(slider, { target: { value: '1' } });
  fireEvent(document, new PointerEvent(type, { bubbles: true, pointerId: 1 }));
  expect(onYear).not.toHaveBeenCalled();
  expect(slider).toHaveValue('0');
});
