import { ChakraProvider } from '@chakra-ui/react';
import type { ReactNode } from 'react';
import { system } from '@/theme';
import { ColorModeProvider } from './color-mode';

export function Provider({ children }: { children: ReactNode }) {
  return (
    <ChakraProvider value={system}>
      <ColorModeProvider>{children}</ColorModeProvider>
    </ChakraProvider>
  );
}
