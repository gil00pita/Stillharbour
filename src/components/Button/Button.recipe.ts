import { defineRecipe } from '@chakra-ui/react';
import { focusRing } from '../../theme/focus-ring';

export const buttonRecipe = defineRecipe({
  className: 'chakra-button',
  base: {
    font: 'inherit',
    fontSize: 'sm',
    lineHeight: 'control',
    fontWeight: 500,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '2',
    flexShrink: 0,
    borderRadius: 'control',
    border: '1px solid',
    py: 'controlY',
    px: '3',
    minHeight: 'control',
    minWidth: 'control',
    cursor: 'pointer',
    transition: 'background-color 140ms var(--ui-ease), border-color 140ms var(--ui-ease)',
    _focusVisible: { ...focusRing, outlineOffset: '5px' },
    _disabled: { cursor: 'default', opacity: 0.55 },
    '@media (prefers-reduced-motion: reduce)': { transition: 'none' },
  },
  variants: {
    variant: {
      outline: {
        bg: 'bg.panel',
        color: 'fg',
        borderColor: 'transparent',
        _hover: { bg: 'bg.hover', borderColor: 'transparent' },
        _active: { bg: 'brand.selected' },
      },
      solid: {
        bg: 'brand.solid',
        color: 'brand.contrast',
        borderColor: 'transparent',
        fontWeight: 600,
        _hover: { bg: 'brand.hover' },
        _active: { bg: 'brand.solid' },
      },
    },
  },
  defaultVariants: { variant: 'outline' },
});
