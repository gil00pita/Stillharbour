import { NativeSelect as ChakraNativeSelect, useRecipe } from '@chakra-ui/react';
import type { ComponentPropsWithRef } from 'react';
import type { SystemStyleObject } from '@chakra-ui/react';
import { nativeSelectRecipe } from './NativeSelect.recipe';

// asChild keeps one native select in the DOM, including its OS arrow and keyboard behavior.
export function NativeSelect({
  size,
  disabled,
  className,
  css,
  ...props
}: ComponentPropsWithRef<'select'> & { css?: SystemStyleObject }) {
  const styles = useRecipe({ recipe: nativeSelectRecipe })();
  return (
    <ChakraNativeSelect.Root asChild unstyled disabled={disabled}>
      <ChakraNativeSelect.Field
        {...props}
        htmlSize={size}
        css={[styles, css]}
        className={['chakra-native-select', className].filter(Boolean).join(' ')}
      />
    </ChakraNativeSelect.Root>
  );
}
