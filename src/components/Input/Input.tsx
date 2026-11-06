import { Input as ChakraInput } from '@chakra-ui/react';
import type { ComponentPropsWithRef } from 'react';

export function Input({ size, ...props }: ComponentPropsWithRef<'input'>) {
  return <ChakraInput {...props} htmlSize={size} />;
}
