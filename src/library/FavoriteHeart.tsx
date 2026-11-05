import { useEffect, useRef } from 'react';
import { useDocumentHidden } from './useDocumentHidden';
export function FavoriteHeart({
  favorite,
  busy,
  animate,
  onClick,
  onAnimationEnd,
}: {
  favorite: boolean;
  busy: boolean;
  animate: boolean;
  onClick: () => void;
  onAnimationEnd: () => void;
}) {
  const hidden = useDocumentHidden();
  const button = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!animate || !favorite) return;
    const value = button.current
      ? getComputedStyle(button.current).getPropertyValue('--like-pop').trim()
      : '';
    // Bundled CSS may serialize milliseconds as seconds (800ms becomes .8s).
    const milliseconds =
      parseFloat(value) * (value.endsWith('ms') ? 1 : value.endsWith('s') ? 1000 : NaN);
    const duration = Number.isFinite(milliseconds) && milliseconds >= 0 ? milliseconds : 800;
    // Occluded WebViews can suspend animationend; always finish at saved state.
    const timer = setTimeout(onAnimationEnd, hidden ? 0 : duration + 70);
    return () => clearTimeout(timer);
  }, [animate, favorite, hidden, onAnimationEnd]);
  return (
    <button
      ref={button}
      className={`viewer-favorite t-like ${animate && favorite ? 'is-animating' : ''} ${hidden ? 'without-motion' : ''}`}
      data-liked={favorite}
      aria-label="Favorite photo"
      aria-pressed={favorite}
      aria-busy={busy}
      title={favorite ? 'Remove from favorites' : 'Add to favorites'}
      disabled={busy}
      onClick={onClick}
    >
      <span className="t-like-icon" aria-hidden="true">
        <span className="t-like-ring" />
        <span className="t-like-particles">
          {Array.from({ length: 7 }, (_, index) => (
            <span
              key={index}
              className="t-like-spark"
              style={{ rotate: `${(index * 360) / 7}deg` }}
            />
          ))}
        </span>
        <svg
          className="t-like-heart"
          onAnimationEnd={(event) => {
            if (event.target === event.currentTarget && event.animationName === 't-like-pop') {
              onAnimationEnd();
            }
          }}
          width="24"
          height="24"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          aria-hidden="true"
        >
          <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0l-1 1-1-1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z" />
        </svg>
      </span>
    </button>
  );
}
