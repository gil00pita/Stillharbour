import { IconButton } from '@chakra-ui/react';
import { useColorMode } from '../../hooks/use-color-mode';

export function AppearanceToggle() {
  const { colorMode, toggleColorMode } = useColorMode();
  const label = colorMode === 'dark' ? 'Switch to light appearance' : 'Switch to dark appearance';
  return (
    <IconButton aria-label={label} title={label} onClick={toggleColorMode} variant="outline">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        {colorMode === 'dark' ? (
          <>
            <circle cx="12" cy="12" r="4" stroke="currentColor" strokeWidth="1.5" />
            <path
              d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </>
        ) : (
          <path
            d="M20.5 14.5A9 9 0 0 1 9.5 3.5a9 9 0 1 0 11 11Z"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        )}
      </svg>
    </IconButton>
  );
}
