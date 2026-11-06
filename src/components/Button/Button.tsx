import { Button as ChakraButton } from '@chakra-ui/react';
import type { ComponentPropsWithRef } from 'react';

export function Button(props: ComponentPropsWithRef<'button'>) {
  return (
    <ChakraButton
      variant={props.className?.split(/\s+/).includes('primary') ? 'solid' : 'outline'}
      {...props}
      type={props.type ?? 'submit'}
    />
  );
}
