import { useTheme } from 'next-themes';

export function useColorMode() {
  const { resolvedTheme, setTheme } = useTheme();
  return {
    colorMode: resolvedTheme ?? 'dark',
    toggleColorMode: () => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark'),
  };
}
