import { chakra } from '@chakra-ui/react';
import type { LayoutGroup } from '../presets';

export function LayoutTypeIcon({ category }: { category: LayoutGroup | 'custom' }) {
  return (
    <chakra.svg
      width="16px"
      height="16px"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {category === 'date' && (
        <>
          <rect x="3" y="5" width="18" height="16" rx="2" />
          <path d="M8 3v4m8-4v4M3 11h18" />
        </>
      )}
      {category === 'location' && (
        <>
          <path d="M20 10c0 6-8 11-8 11S4 16 4 10a8 8 0 1 1 16 0Z" />
          <circle cx="12" cy="10" r="3" />
        </>
      )}
      {category === 'tags' && (
        <>
          <path d="M3 3h8l10 10-8 8L3 11V3Z" />
          <circle cx="7.5" cy="7.5" r="1" />
        </>
      )}
      {category === 'camera' && (
        <>
          <path d="M7 6 9 3h6l2 3h3a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h3Z" />
          <circle cx="12" cy="12" r="4" />
        </>
      )}
      {category === 'custom' && (
        <>
          <path d="M3 6h5m4 0h9M3 12h9m4 0h5M3 18h3m4 0h11" />
          <circle cx="10" cy="6" r="2" />
          <circle cx="14" cy="12" r="2" />
          <circle cx="8" cy="18" r="2" />
        </>
      )}
    </chakra.svg>
  );
}
