import { defineRecipe } from '@chakra-ui/react';
export const pageHeaderRecipe = defineRecipe({
  base: {
    height: 'appHeader',
    minHeight: 'appHeader',
    maxHeight: 'appHeader',
    flexShrink: 0,
    boxSizing: 'border-box',
    '& h1': { userSelect: 'none', fontSize: 'lg', lineHeight: 'title', margin: 0 },
  },
});
