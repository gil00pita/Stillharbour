import {
  Button as ChakraButton,
  Input as ChakraInput,
  Textarea as ChakraTextarea,
  chakra,
} from '@chakra-ui/react';
import type { ComponentPropsWithRef } from 'react';
import { fieldRecipe } from '../../theme/recipes';

// Keep native event/ref/form contracts at the product boundary.
export function Button(props: ComponentPropsWithRef<'button'>) {
  return (
    <ChakraButton
      variant={props.className?.split(/\s+/).includes('primary') ? 'solid' : 'outline'}
      {...props}
      type={props.type ?? 'submit'}
    />
  );
}
export function Input(props: ComponentPropsWithRef<'input'>) {
  const { size, ...rest } = props;
  return <ChakraInput {...rest} htmlSize={size} />;
}
export function Textarea(props: ComponentPropsWithRef<'textarea'>) {
  return <ChakraTextarea {...props} />;
}
// Preserve the native select, including OS keyboard navigation and form values.
// The factory uses the same recipe as text fields without inserting wrapper DOM.
const StyledSelect = chakra('select', {
  ...fieldRecipe,
  className: 'chakra-native-select',
  base: { ...fieldRecipe.base, minHeight: 'control', paddingRight: '36px' },
});
export function Select(props: ComponentPropsWithRef<'select'>) {
  const { size, className, ...rest } = props;
  return (
    <StyledSelect
      {...rest}
      className={['chakra-native-select', className].filter(Boolean).join(' ')}
      htmlSize={size}
    />
  );
}
