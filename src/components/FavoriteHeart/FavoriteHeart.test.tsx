import { act, fireEvent, render, screen } from '../../test/render';
import { afterEach, expect, it, vi } from 'vitest';
import { FavoriteHeart } from '@/components/FavoriteHeart';

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

it.each(['800ms', '.8s'])(
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
    act(() => vi.advanceTimersByTime(799));
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

it('only completes the burst on the heart animation, not its decorative layers', () => {
  const complete = vi.fn();
  const { container } = render(
    <FavoriteHeart favorite busy={false} animate onClick={vi.fn()} onAnimationEnd={complete} />,
  );
  fireEvent.animationEnd(container.querySelector('.t-like-ring')!, {
    animationName: 't-like-ring',
  });
  expect(complete).not.toHaveBeenCalled();
  const event = new Event('AnimationEvent' in window ? 'animationend' : 'webkitAnimationEnd', {
    bubbles: true,
  });
  Object.defineProperty(event, 'animationName', { value: 't-like-pop' });
  fireEvent(container.querySelector('.t-like-heart')!, event);
  expect(complete).toHaveBeenCalledOnce();
});

it('never celebrates an unfavorite or a stored favorite on initial display', () => {
  const { rerender } = render(
    <FavoriteHeart
      favorite={false}
      busy={false}
      animate
      onClick={vi.fn()}
      onAnimationEnd={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Favorite photo' })).not.toHaveClass('is-animating');
  rerender(
    <FavoriteHeart
      favorite
      busy={false}
      animate={false}
      onClick={vi.fn()}
      onAnimationEnd={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Favorite photo' })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  expect(screen.getByRole('button', { name: 'Favorite photo' })).not.toHaveClass('is-animating');
});
