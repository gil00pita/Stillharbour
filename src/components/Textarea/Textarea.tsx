import { Textarea as ChakraTextarea } from '@chakra-ui/react';
import type { ComponentPropsWithRef } from 'react';

export function Textarea(props: ComponentPropsWithRef<'textarea'>) {
  return <ChakraTextarea {...props} />;
}
