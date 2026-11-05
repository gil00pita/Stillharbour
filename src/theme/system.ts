import { createSystem, defaultBaseConfig, defineConfig, defineRecipe } from '@chakra-ui/react';
import { fieldRecipe } from './recipes';
import { placeSearchRecipe } from '../components/PlaceSearch/PlaceSearch.slot-recipe';

const focus = { outline: '2px solid', outlineColor: 'brand.focusRing', outlineOffset: '2px' };

export const system = createSystem(
  defaultBaseConfig,
  defineConfig({
    cssVarsPrefix: 'stillharbour',
    // Retain the desktop's native form, dialog and photographic canvas behavior.
    preflight: false,
    theme: {
      tokens: {
        spacing: {
          1: { value: '0.25rem' },
          2: { value: '0.5rem' },
          3: { value: '0.75rem' },
          4: { value: '1rem' },
        },
        fontSizes: { xs: { value: '12px' }, sm: { value: '13px' } },
        zIndex: { popover: { value: 1500 } },
        colors: {
          harbour: {
            canvas: { value: '#16191b' },
            panel: { value: '#1d2123' },
            input: { value: '#161a1c' },
            ink: { value: '#e7e9e8' },
            muted: { value: '#a9b3b2' },
            secondary: { value: '#c7cecb' },
            border: { value: '#303638' },
            controlBorder: { value: '#46504f' },
            buttonBorder: { value: '#3c4643' },
            hover: { value: '#232827' },
            accent: { value: '#b6d6c8' },
            accentHover: { value: '#cee5da' },
            onAccent: { value: '#162b22' },
            selected: { value: '#3c5147' },
            danger: { value: '#f2c5b4' },
            dangerBg: { value: '#30231e' },
          },
        },
        fonts: {
          body: { value: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
          heading: { value: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
        },
        radii: { control: { value: '8px' }, panel: { value: '12px' } },
        sizes: {
          control: { value: '36px' },
          full: { value: '100%' },
          sm: { value: '24rem' },
          64: { value: '16rem' },
        },
      },
      semanticTokens: {
        colors: {
          bg: {
            DEFAULT: { value: '{colors.harbour.canvas}' },
            panel: { value: '{colors.harbour.panel}' },
            input: { value: '{colors.harbour.input}' },
            hover: { value: '{colors.harbour.hover}' },
          },
          fg: {
            DEFAULT: { value: '{colors.harbour.ink}' },
            muted: { value: '{colors.harbour.muted}' },
            secondary: { value: '{colors.harbour.secondary}' },
          },
          border: {
            DEFAULT: { value: '{colors.harbour.border}' },
            control: { value: '{colors.harbour.controlBorder}' },
            button: { value: '{colors.harbour.buttonBorder}' },
          },
          brand: {
            solid: { value: '{colors.harbour.accent}' },
            contrast: { value: '{colors.harbour.onAccent}' },
            hover: { value: '{colors.harbour.accentHover}' },
            focusRing: { value: '{colors.harbour.accent}' },
            selected: { value: '{colors.harbour.selected}' },
          },
          danger: {
            fg: { value: '{colors.harbour.danger}' },
            bg: { value: '{colors.harbour.dangerBg}' },
          },
        },
      },
      recipes: {
        button: defineRecipe({
          className: 'chakra-button',
          base: {
            font: 'inherit',
            fontSize: '13px',
            lineHeight: '20px',
            fontWeight: 400,
            borderRadius: 'control',
            border: '1px solid',
            padding: '7px 12px',
            minHeight: 'control',
            cursor: 'pointer',
            transition: 'background-color 140ms var(--ui-ease), border-color 140ms var(--ui-ease)',
            _focusVisible: { ...focus, outlineOffset: '5px' },
            _disabled: { cursor: 'default', opacity: 0.55 },
            '@media (prefers-reduced-motion: reduce)': { transition: 'none' },
          },
          variants: {
            variant: {
              outline: {
                bg: 'bg.panel',
                color: 'fg',
                borderColor: 'transparent',
                _hover: { bg: 'bg.hover' },
              },
              solid: {
                bg: 'brand.solid',
                color: 'brand.contrast',
                borderColor: 'transparent',
                fontWeight: 600,
                _hover: { bg: 'brand.hover' },
              },
            },
          },
          defaultVariants: { variant: 'outline' },
        }),
        input: {
          ...fieldRecipe,
          className: 'chakra-input',
          base: { ...fieldRecipe.base, minHeight: 'control' },
        },
        textarea: { ...fieldRecipe, className: 'chakra-textarea' },
      },
      slotRecipes: { popover: placeSearchRecipe },
    },
  }),
);
