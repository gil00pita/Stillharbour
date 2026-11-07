import { defineSlotRecipe } from '@chakra-ui/react';
import { focusRing } from '../../theme/focus-ring';

export const segmentedControlRecipe = defineSlotRecipe({
  slots: ['root', 'item', 'itemText'],
  base: {
    root: {
      display: 'flex',
      minWidth: 0,
      padding: '1',
      gap: '1',
      border: '1px solid',
      borderColor: 'border.control',
      borderRadius: 'control',
      bg: 'bg.input',
    },
    item: {
      position: 'relative',
      flex: '1',
      minWidth: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: '0',
      margin: 0,
      padding: '2',
      minHeight: 'control',
      borderRadius: 'control',
      cursor: 'pointer',
      color: 'fg.muted',
      _checked: { bg: 'brand.selected', color: 'fg' },
      _hover: { bg: 'bg.hover', color: 'fg' },
      _focus: { ...focusRing },
      _disabled: { opacity: 0.55, cursor: 'default' },
      '.organization-body &': { display: 'flex', gap: '0' },
    },
    itemText: { fontSize: 'sm', lineHeight: 'control', textAlign: 'center', fontWeight: 500 },
  },
});
