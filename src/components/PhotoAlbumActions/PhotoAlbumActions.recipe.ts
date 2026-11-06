import { defineRecipe } from '@chakra-ui/react';
export const photoAlbumActionsRecipe = defineRecipe({
  base: {
    '& .album-chooser': {
      display: 'flex',
      flexDirection: 'column',
      gap: '12px',
      padding: '24px',
      overflow: 'auto',
    },
    '& .album-chooser > button': { justifyContent: 'space-between', flexShrink: 0 },
    '& .album-chooser > button span': { color: 'fg.muted', fontSize: '12px' },
  },
});
