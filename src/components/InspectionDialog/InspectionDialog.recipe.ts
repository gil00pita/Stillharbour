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
    '& > .inspection-dialog-footer': {
      flexShrink: 0,
      display: 'flex',
      alignItems: 'center',
      gap: '12px',
      flexWrap: 'wrap',
      padding: '16px 24px',
      borderTop: '1px solid {colors.border}',
      background: 'bg.panel',
    },
    '& > .inspection-dialog-footer > span': { marginRight: 'auto', color: 'fg.muted' },
  },
});
