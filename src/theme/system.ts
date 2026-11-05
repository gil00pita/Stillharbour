import {
  createSystem,
  defaultBaseConfig,
  defaultConfig,
  defineConfig,
  defineRecipe,
} from '@chakra-ui/react';
import { fieldRecipe } from './recipes';
import { placeSearchRecipe } from '../components/PlaceSearch/PlaceSearch.slot-recipe';
import { inspectionChoiceCardsRecipe } from '../components/InspectionChoiceCards/InspectionChoiceCards.slot-recipe';
import { errorToastRecipe } from '../components/ErrorToast/ErrorToast.slot-recipe';
import { storageExplorerRecipe } from '../components/StorageExplorer/StorageExplorer.recipe';

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
            canvas: { value: '#191b19' },
            sidebar: { value: '#131612' },
            toolbar: { value: '#1c1f1b' },
            navigation: { value: '#2b3327' },
            panel: { value: '#212420' },
            input: { value: '#171a17' },
            ink: { value: '#eeeee8' },
            muted: { value: '#a9afa4' },
            secondary: { value: '#c8ccc2' },
            border: { value: '#323830' },
            controlBorder: { value: '#6d7567' },
            buttonBorder: { value: '#687161' },
            hover: { value: '#2b3029' },
            accent: { value: '#c1d0b5' },
            accentHover: { value: '#d4dfcb' },
            onAccent: { value: '#202a1b' },
            selected: { value: '#374231' },
            danger: { value: '#f2c5b4' },
            dangerBg: { value: '#30231e' },
          },
        },
        fonts: {
          body: { value: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
          heading: { value: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif' },
        },
        radii: { control: { value: '6px' }, panel: { value: '12px' } },
        sizes: {
          control: { value: '36px' },
          full: { value: '100%' },
          sm: { value: '24rem' },
          toast: { value: '34rem' },
          64: { value: '16rem' },
        },
      },
      semanticTokens: {
        colors: {
          bg: {
            DEFAULT: { value: '{colors.harbour.canvas}' },
            panel: { value: '{colors.harbour.panel}' },
            sidebar: { value: '{colors.harbour.sidebar}' },
            toolbar: { value: '{colors.harbour.toolbar}' },
            navigation: { value: '{colors.harbour.navigation}' },
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
      slotRecipes: {
        field: defaultConfig.theme!.slotRecipes!.field!,
        radioCard: inspectionChoiceCardsRecipe,
        popover: placeSearchRecipe,
        toast: errorToastRecipe,
      },
      recipes: {
        storageExplorer: storageExplorerRecipe,
        button: defineRecipe({
          className: 'chakra-button',
          base: {
            font: 'inherit',
            fontSize: '13px',
            lineHeight: '20px',
            fontWeight: 500,
            display: 'inline-flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '8px',
            flexShrink: 0,
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
                _hover: { bg: 'bg.hover', borderColor: 'transparent' },
                _active: { bg: 'brand.selected' },
              },
              solid: {
                bg: 'brand.solid',
                color: 'brand.contrast',
                borderColor: 'transparent',
                fontWeight: 600,
                _hover: { bg: 'brand.hover' },
                _active: { bg: 'brand.solid' },
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
    },
  }),
);
