import { defineSlotRecipe } from '@chakra-ui/react';

export const placeSearchRecipe = defineSlotRecipe({
  slots: ['positioner', 'content', 'body', 'title'],
  base: {
    positioner: { zIndex: 'popover' },
    content: {
      _closed: { display: 'none' },
      bg: 'bg.panel',
      color: 'fg',
      fontSize: 'sm',
      borderWidth: '1px',
      borderColor: 'border.control',
      borderRadius: 'panel',
      boxShadow: '0 8px 32px {colors.harbour.canvas}',
    },
    body: { p: '4' },
    title: { fontSize: 'sm', fontWeight: '600' },
  },
});
