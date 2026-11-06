import { defineRecipe } from '@chakra-ui/react';

export const inspectionDialogRecipe = defineRecipe({
  base: {
    width: 'min(760px, calc(100vw - 32px))',
    maxHeight: 'calc(100dvh - 48px)',
    padding: 0,
    margin: 'auto',
    border: '1px solid var(--border)',
    borderRadius: 'panel',
    background: 'var(--surface)',
    color: 'inherit',
    '&[open]': { display: 'flex', flexDirection: 'column' },
    '&::backdrop': { background: 'rgb(0 0 0 / 65%)' },
    '& > header': {
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '16px',
      padding: '16px 24px',
      borderBottom: '1px solid var(--border)',
      flexShrink: 0,
    },
    '& > header h2': { fontSize: '18px', margin: 0 },
    '& .inspection-content': { paddingTop: 0 },
  },
});
