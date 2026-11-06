import { defineRecipe } from '@chakra-ui/react';

import { focusRing } from '../../theme/focus-ring';
export const fieldRecipe = defineRecipe({
  base: {
    font: 'inherit',
    fontSize: 'sm',
    lineHeight: 'control',
    color: 'fg',
    backgroundColor: 'bg.input',
    border: '1px solid',
    borderColor: 'border.control',
    borderRadius: 'control',
    py: 'controlY',
    px: '3',
    minWidth: '0',
    width: '100%',
    _placeholder: { color: 'fg.muted', opacity: 1 },
    _focusVisible: focusRing,
    _disabled: { opacity: 0.55, cursor: 'default' },
  },
});

export const inputRecipe = defineRecipe({
  ...fieldRecipe,
  className: 'chakra-input',
  base: { ...fieldRecipe.base, minHeight: 'control' },
});
