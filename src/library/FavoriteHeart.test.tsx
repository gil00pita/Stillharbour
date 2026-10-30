import { act, render } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { FavoriteHeart } from './FavoriteHeart';

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

it.each(['180ms', '.18s'])(
  'keeps confirmed feedback through its duration when CSS reports %s',
  (value) => {
    vi.useFakeTimers();
    vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('visible');
    const style = document.createElement('div').style;
    style.setProperty('--like-pop', value);
    vi.spyOn(globalThis, 'getComputedStyle').mockReturnValue(style);
    const complete = vi.fn();
    render(
      <FavoriteHeart favorite busy={false} animate onClick={vi.fn()} onAnimationEnd={complete} />,
    );
    act(() => vi.advanceTimersByTime(179));
    expect(complete).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(70));
    expect(complete).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));
    expect(complete).toHaveBeenCalledOnce();
  },
);

it('finishes feedback immediately in a hidden native window', () => {
  vi.useFakeTimers();
  vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
  const complete = vi.fn();
  render(
    <FavoriteHeart favorite busy={false} animate onClick={vi.fn()} onAnimationEnd={complete} />,
  );
  act(() => vi.advanceTimersByTime(0));
  expect(complete).toHaveBeenCalledOnce();
});
