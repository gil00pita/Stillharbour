import { Button as ChakraButton } from '@chakra-ui/react';
import type { ButtonProps } from '@chakra-ui/react';
import type { RefAttributes } from 'react';

export function Button(props: ButtonProps & RefAttributes<HTMLButtonElement>) {
  return (
    <ChakraButton
      variant={props.className?.split(/\s+/).includes('primary') ? 'solid' : 'outline'}
      {...props}
      type={props.type ?? 'submit'}
    />
  );
}
