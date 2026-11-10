import { Textarea as ChakraTextarea } from '@chakra-ui/react';
import type { TextareaProps } from '@chakra-ui/react';
import type { RefAttributes } from 'react';

export function Textarea(props: TextareaProps & RefAttributes<HTMLTextAreaElement>) {
  return <ChakraTextarea {...props} />;
}
