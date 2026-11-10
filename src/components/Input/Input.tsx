import { Input as ChakraInput } from '@chakra-ui/react';
import type { InputProps } from '@chakra-ui/react';
import type { RefAttributes } from 'react';

export function Input(
  props: Omit<InputProps, 'size'> & RefAttributes<HTMLInputElement> & { size?: number },
) {
  const { size, ...rest } = props;
  return <ChakraInput {...rest} htmlSize={size} />;
}
