// Shared animation definitions referenced by component styles.
export const keyframes = {
  'photo-scan-search': {
    '0%,\n  100%': {
      transform: 'translate(-34px, -18px)',
    },
    '35%': {
      transform: 'translate(30px, -12px)',
    },
    '65%': {
      transform: 'translate(-20px, 24px)',
    },
    '85%': {
      transform: 'translate(24px, 16px)',
    },
  },
  'photo-scan-float': {
    '0%,\n  100%': {
      transform: 'translateY(0)',
    },
    '50%': {
      transform: 'translateY(-4px)',
    },
  },
  'scan-activity': {
    from: {
      transform: 'translateX(0)',
    },
    to: {
      transform: 'translateX(212.5%)',
    },
  },
  't-skel-pulse': {
    '0%,\n  100%': {
      opacity: '1',
    },
    '50%': {
      opacity: 'var(--pulse-min)',
    },
  },
  'preview-loading-glare': {
    to: {
      transform: 'translateX(110%)',
    },
  },
  't-like-pop': {
    '0%,\n  15%': {
      opacity: '0',
      transform: 'scale(0.15)',
    },
    '38%': {
      opacity: '1',
      transform: 'scale(1.22)',
    },
    '52%': {
      transform: 'scale(0.94)',
    },
    '68%,\n  100%': {
      opacity: '1',
      transform: 'scale(1)',
    },
  },
  't-like-ring': {
    '0%': {
      opacity: '0.9',
      borderWidth: '8px',
      transform: 'scale(0.15)',
    },
    '24%': {
      opacity: '0.7',
      borderWidth: '3px',
      transform: 'scale(1)',
    },
    '42%,\n  100%': {
      opacity: '0',
      borderWidth: '0',
      transform: 'scale(1.35)',
    },
  },
  't-like-sparks': {
    '0%,\n  15%': {
      opacity: '0',
      transform: 'translate(-2px, -2px) scale(0.2)',
    },
    '25%': {
      opacity: '1',
    },
    '65%': {
      opacity: '0.85',
      transform: 'translate(var(--spark-offset), calc(-1 * var(--spark-distance))) scale(1)',
    },
    '100%': {
      opacity: '0',
      transform: 'translate(var(--spark-offset), calc(-1.25 * var(--spark-distance))) scale(0)',
    },
  },
};
