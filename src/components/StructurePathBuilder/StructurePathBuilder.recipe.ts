import { defineRecipe } from '@chakra-ui/react';
import { focusRing } from '../../theme/focus-ring';

export const structurePathBuilderRecipe = defineRecipe({
  base: {
    marginTop: '4',
    minWidth: 0,
    '& .structure-path-input': {
      display: 'flex',
      flexWrap: 'wrap',
      alignItems: 'center',
      gap: '2',
      padding: '2',
      border: '1px solid',
      borderColor: 'border.control',
      borderRadius: 'control',
      bg: 'bg.input',
      _focusWithin: { ...focusRing },
    },
    '& .structure-path-input input': {
      flex: '1 1 11rem',
      width: 'auto',
      minWidth: 0,
      border: 0,
      bg: 'transparent',
    },
    '& .structure-path-chips': { display: 'contents', listStyle: 'none', padding: 0, margin: 0 },
    '& .structure-path-chip': {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '1',
      borderRadius: 'control',
      bg: 'brand.selected',
      paddingLeft: '2',
      color: 'fg',
    },
    '& .structure-path-chip button': {
      padding: '1',
      minHeight: 'control',
      minWidth: 'control',
      border: 0,
    },
    '& .structure-part-options': {
      display: 'flex',
      flexWrap: 'wrap',
      gap: '2',
      padding: 0,
      listStyle: 'none',
      marginBlock: '3',
      maxHeight: '14rem',
      overflowY: 'auto',
    },
    '& .structure-part-options [aria-selected=true]': {
      outline: '2px solid',
      outlineColor: 'brand.focusRing',
      outlineOffset: '2px',
    },
    '& .structure-path-example': {
      padding: '3',
      borderRadius: 'control',
      bg: 'bg.input',
      color: 'fg.secondary',
      overflowWrap: 'anywhere',
      fontVariantNumeric: 'tabular-nums',
    },
  },
});
