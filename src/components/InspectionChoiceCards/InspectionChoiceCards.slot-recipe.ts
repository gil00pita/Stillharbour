import { defineSlotRecipe } from '@chakra-ui/react';

export const inspectionChoiceCardsRecipe = defineSlotRecipe({
  slots: [
    'root',
    'label',
    'item',
    'itemControl',
    'itemContent',
    'itemText',
    'itemDescription',
    'itemIndicator',
  ],
  base: {
    root: {
      display: 'block',
      minWidth: 0,
      containerType: 'inline-size',
      '& .inspection-choice-grid': { display: 'grid', gap: '12px' },
      '& .inspection-choice-badge': {
        display: 'block',
        marginTop: '12px',
        fontSize: '12px',
        lineHeight: '18px',
        fontWeight: 550,
        color: 'fg.secondary',
      },
    },
    label: {
      display: 'block',
      marginBottom: '12px',
      fontSize: '14px',
      lineHeight: '20px',
      fontWeight: 600,
      color: 'fg',
    },
    item: {
      position: 'relative',
      display: 'flex',
      minWidth: 0,
      margin: 0,
      cursor: 'pointer',
      _disabled: { opacity: 0.55, cursor: 'default' },
    },
    itemControl: {
      display: 'grid',
      alignContent: 'start',
      gap: '16px',
      width: '100%',
      minWidth: 0,
      padding: '20px',
      border: '1px solid',
      borderColor: 'border.button',
      borderRadius: 'panel',
      background: 'bg',
      color: 'fg',
      transition: 'background-color 140ms var(--ui-ease), border-color 140ms var(--ui-ease)',
      _hover: { background: 'bg.hover', borderColor: 'border.control' },
      _checked: { background: 'bg.navigation', borderColor: 'brand.solid' },
      _focus: { outline: '2px solid', outlineColor: 'brand.focusRing', outlineOffset: '3px' },
      '& > img': { display: 'block', width: '100%', height: '140px', objectFit: 'contain' },
      '@media (prefers-reduced-motion: reduce)': { transition: 'none' },
    },
    itemContent: { display: 'flex', flexDirection: 'column', gap: '8px', minWidth: 0 },
    itemText: {
      fontSize: '15px',
      lineHeight: '22px',
      fontWeight: 600,
      color: 'fg',
      paddingRight: '16px',
    },
    itemDescription: { fontSize: '13px', lineHeight: '20px', color: 'fg.muted' },
    itemIndicator: {
      position: 'absolute',
      top: '16px',
      right: '16px',
      width: '18px',
      height: '18px',
      borderRadius: '50%',
      display: 'grid',
      placeItems: 'center',
      border: '1px solid',
      borderColor: 'border.control',
      color: 'brand.contrast',
      _checked: { background: 'brand.solid', borderColor: 'brand.solid' },
      '& .dot': { width: '6px', height: '6px', background: 'currentColor', borderRadius: '50%' },
    },
  },
  variants: {
    presentation: {
      illustrated: {
        root: {
          '& .inspection-choice-grid': { gridTemplateColumns: 'repeat(3, minmax(0, 1fr))' },
          '@container (max-width: 620px)': {
            '& .inspection-choice-grid': { gridTemplateColumns: 'minmax(0, 1fr)' },
            '& .inspection-choice-card > div': {
              gridTemplateColumns: '88px minmax(0, 1fr)',
              gap: '16px',
              padding: '16px',
            },
            '& .inspection-choice-card > div > img': {
              height: '100%',
              maxHeight: '120px',
              alignSelf: 'center',
            },
          },
        },
      },
      compact: {
        root: {
          '& .inspection-choice-grid': { gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' },
          '@container (max-width: 420px)': {
            '& .inspection-choice-grid': { gridTemplateColumns: 'minmax(0, 1fr)' },
          },
        },
        itemControl: { padding: '16px' },
        itemText: { fontSize: '13px', lineHeight: '20px', paddingRight: '24px' },
      },
      inline: {
        root: {
          '& .inspection-choice-grid': {
            gridTemplateColumns: 'repeat(auto-fit, minmax(138px, 1fr))',
            gap: '8px',
          },
        },
        itemControl: { padding: '12px', gap: '8px' },
        itemText: { fontSize: '13px', lineHeight: '20px', paddingRight: '22px' },
        itemIndicator: { top: '13px', right: '12px', width: '16px', height: '16px' },
      },
    },
  },
  defaultVariants: { presentation: 'compact' },
});
