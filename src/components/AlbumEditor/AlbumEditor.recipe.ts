import { defineRecipe } from '@chakra-ui/react';
export const albumEditorRecipe = defineRecipe({
  base: {
    padding: '4',
    overflow: 'auto',
    minHeight: 0,
    '& form': { display: 'flex', flexDirection: 'column', gap: '16px' },
    '& .album-hint': { color: 'fg.muted', margin: 0, lineHeight: 1.6 },
    '& .album-picker': { height: 'min(34dvh, 300px)', minHeight: '160px', display: 'flex' },
    '& .browser-frame': { width: '100%', minWidth: 0 },
  },
});
