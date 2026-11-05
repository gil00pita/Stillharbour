import { defineSlotRecipe } from '@chakra-ui/react';

export const errorToastRecipe = defineSlotRecipe({
  slots: ['root', 'title', 'description', 'indicator', 'actionTrigger', 'closeTrigger'],
  className: 'harbour-toast',
  base: {
    root: {
      display: 'grid',
      gridTemplateColumns: 'auto minmax(0, 1fr) auto',
      alignItems: 'start',
      gap: '3',
      p: '4',
      width: 'full',
      maxWidth: 'toast',
      bg: 'bg.panel',
      color: 'fg',
      borderWidth: '1px',
      borderStyle: 'solid',
      borderColor: 'border.control',
      borderRadius: 'panel',
      fontFamily: 'body',
      _focusVisible: { outline: '2px solid {colors.brand.focusRing}', outlineOffset: '2px' },
    },
    title: { fontSize: 'sm', fontWeight: 600, lineHeight: 1.5 },
    description: {
      fontSize: 'sm',
      color: 'fg.secondary',
      lineHeight: 1.6,
      whiteSpace: 'pre-wrap',
      overflowWrap: 'anywhere',
      userSelect: 'text',
      maxHeight: '40vh',
      overflowY: 'auto',
      _focusVisible: { outline: '2px solid {colors.brand.focusRing}', outlineOffset: '2px' },
    },
    indicator: { boxSize: '1.25rem', color: 'danger.fg', mt: '1', flexShrink: 0 },
    closeTrigger: { width: 'control', minWidth: 'control', '& svg': { boxSize: '1rem' } },
  },
});
