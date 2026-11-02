import { defineRecipe } from '@chakra-ui/react';

const focus = { outline: '2px solid', outlineColor: 'brand.focusRing', outlineOffset: '2px' };
export const fieldRecipe = defineRecipe({
  base: {
    font: 'inherit',
    fontSize: '13px',
    lineHeight: '20px',
    color: 'fg',
    backgroundColor: 'bg.input',
    border: '1px solid',
    borderColor: 'border.control',
    borderRadius: 'control',
    padding: '7px 12px',
    minWidth: '0',
    width: '100%',
    _placeholder: { color: 'fg.muted', opacity: 1 },
    _focusVisible: focus,
    _disabled: { opacity: 0.55, cursor: 'default' },
  },
});
