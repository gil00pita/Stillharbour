import { ThemeProvider } from 'next-themes';
import type { ThemeProviderProps } from 'next-themes';

// Chakra's next-themes composition, shared by the desktop and its portals.
export function ColorModeProvider(props: ThemeProviderProps) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      storageKey="stillharbour-appearance"
      disableTransitionOnChange
      // This app is client rendered. The effect applies the theme; keep the
      // SSR bootstrap inert under the desktop's strict script CSP.
      scriptProps={{ type: 'application/json' }}
      {...props}
    />
  );
}
