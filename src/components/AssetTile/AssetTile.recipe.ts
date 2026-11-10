import { defineStyle } from '@chakra-ui/react';

export const thumbnailControlStyles = defineStyle({
  position: 'absolute',
  top: '2',
  zIndex: 3,
  display: 'grid',
  placeItems: 'center',
  width: 'control',
  height: 'control',
  minWidth: 'control',
  minHeight: 'control',
  margin: 0,
  padding: 0,
  border: 0,
  borderRadius: '50%',
  background: 'transparent',
  color: 'photo.controlFg',
  cursor: 'pointer',
  _disabled: { cursor: 'default' },
  _focusVisible: {
    outline: '2px solid {colors.photo.controlFg}',
    outlineOffset: 0,
    boxShadow: '0 0 0 4px {colors.photo.controlInk}',
  },
});

export const thumbnailMarkStyles = defineStyle({
  display: 'grid',
  placeItems: 'center',
  width: '28px',
  height: '28px',
  border: '1px solid transparent',
  borderRadius: '50%',
  background: 'photo.controlBg',
  color: 'photo.controlFg',
  pointerEvents: 'none',
  '& svg': { display: 'block', width: '18px', height: '18px' },
  '& .selection-tick': { stroke: 'photo.controlInk' },
  '@media (forced-colors: active)': {
    forcedColorAdjust: 'none',
    background: 'Canvas !important',
    color: 'CanvasText',
    borderColor: 'CanvasText',
    '& .selection-tick': { stroke: 'Canvas' },
  },
});
