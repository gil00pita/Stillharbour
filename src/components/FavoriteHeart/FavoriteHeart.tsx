import { chakra } from '@chakra-ui/react';
import { Text, IconButton } from '@chakra-ui/react';
import { useEffect, useRef } from 'react';
import { useDocumentHidden } from '@/hooks/use-document-hidden';
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
    <IconButton
      unstyled
      type="submit"
      ref={button}
      className={`viewer-favorite t-like ${animate && favorite ? 'is-animating' : ''} ${hidden ? 'without-motion' : ''}`}
      css={{
        "&[data-liked='true'] .t-like-heart": { color: 'var(--like-color)' },
        "&[data-liked='true'] .t-like-heart path": { fill: 'currentColor' },
        "&.is-animating[data-liked='true'] .t-like-heart": {
          animation: 't-like-pop var(--like-pop) var(--like-pop-ease)',
        },
        "&.is-animating[data-liked='true'] .t-like-ring": {
          animation: 't-like-ring var(--like-pop) ease-out',
        },
        "&.is-animating[data-liked='true'] .t-like-spark::before": {
          animation: 't-like-sparks var(--like-pop) ease-out',
        },
        "&.is-animating[data-liked='true'] .t-like-spark::after": {
          animation: 't-like-sparks var(--like-pop) ease-out',
        },
        '&:not(.is-animating) .t-like-heart': { transition: 'none' },
        '&:not(.is-animating) .t-like-heart path': { transition: 'none' },
        '&.without-motion .t-like-heart': { animation: 'none !important', transition: 'none' },
        '&.without-motion .t-like-ring': { animation: 'none !important' },
        '&.without-motion .t-like-spark::before': { animation: 'none !important' },
        '&.without-motion .t-like-spark::after': { animation: 'none !important' },
        '&.without-motion .t-like-heart path': { transition: 'none' },
      }}
      data-liked={favorite}
      aria-label="Favorite photo"
      aria-pressed={favorite}
      aria-busy={busy}
      title={favorite ? 'Remove from favorites' : 'Add to favorites'}
      disabled={busy}
      onClick={onClick}
    >
      <Text
        as="span"
        className="t-like-icon"
        css={{
          position: 'relative',
          display: 'inline-flex',
          width: '24px',
          height: '24px',
          pointerEvents: 'none',
        }}
        aria-hidden="true"
      >
        <Text
          as="span"
          className="t-like-ring"
          css={{
            position: 'absolute',
            inset: '-8px',
            border: '0 solid var(--like-color)',
            borderRadius: '50%',
            opacity: '0',
            '@media (prefers-reduced-motion: reduce)': { animation: 'none !important' },
          }}
        />
        <Text as="span" className="t-like-particles" css={{ position: 'absolute', inset: '0' }}>
          {Array.from({ length: 7 }, (_, index) => (
            <Text
              as="span"
              key={index}
              className="t-like-spark"
              css={{
                position: 'absolute',
                inset: '0',
                '--spark-color': '{colors.fg.secondary}',
                '&:nth-child(2n)': { '--spark-color': '{colors.brand.solid}' },
                '&:nth-child(3n)': { '--spark-color': '{colors.fg.secondary}' },
                '&:nth-child(4n)': { '--spark-color': '{colors.fg.secondary}' },
                '&::before': {
                  '--spark-distance': '27px',
                  '--spark-offset': '-2px',
                  content: "''",
                  position: 'absolute',
                  top: '50%',
                  left: '50%',
                  width: '4px',
                  height: '4px',
                  margin: '-2px',
                  borderRadius: '50%',
                  background: 'var(--spark-color)',
                  opacity: '0',
                },
                '&::after': {
                  '--spark-distance': '21px',
                  '--spark-offset': '4px',
                  content: "''",
                  position: 'absolute',
                  top: '50%',
                  left: '50%',
                  width: '3px',
                  height: '3px',
                  margin: '-2px',
                  borderRadius: '50%',
                  background: 'var(--spark-color)',
                  opacity: '0',
                },
                '@media (prefers-reduced-motion: reduce)': {
                  '&::before': { animation: 'none !important' },
                  '&::after': { animation: 'none !important' },
                },
              }}
              style={{ rotate: `${(index * 360) / 7}deg` }}
            />
          ))}
        </Text>
        <chakra.svg
          className="t-like-heart"
          css={{
            position: 'relative',
            color: 'currentColor',
            transformOrigin: 'center',
            transition: 'color var(--like-fill) var(--like-ease)',
            '& path': {
              fill: 'transparent',
              stroke: 'currentColor',
              transition:
                'fill var(--like-fill) var(--like-ease),\n    stroke var(--like-fill) var(--like-ease)',
            },
            '@media (prefers-reduced-motion: reduce)': {
              animation: 'none !important',
              transition: 'none !important',
              '& path': { transition: 'none !important' },
            },
          }}
          onAnimationEnd={(event) => {
            if (event.target === event.currentTarget && event.animationName === 't-like-pop') {
              onAnimationEnd();
            }
          }}
          width="24px"
          height="24px"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          aria-hidden="true"
        >
          <path d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0l-1 1-1-1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z" />
        </chakra.svg>
      </Text>
    </IconButton>
  );
}
