import { defineRecipe } from '@chakra-ui/react';
import { fieldRecipe } from '../Input/Input.recipe';

export const nativeSelectRecipe = defineRecipe({
  ...fieldRecipe,
  base: { ...fieldRecipe.base, minHeight: 'control', paddingRight: 'control' },
});
