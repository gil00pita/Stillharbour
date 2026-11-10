import { structurePathBuilderRecipe } from '../components/StructurePathBuilder/StructurePathBuilder.recipe';
import { structureFormatterRecipe } from '../components/StructureFormatter/StructureFormatter.recipe';
import { structureDialogRecipe } from '../components/StructureFormatter/StructureDialog.slot-recipe';
import { organizationPresetsRecipe } from '../components/OrganizationPresets/OrganizationPresets.slot-recipe';
import { segmentedControlRecipe } from '../components/SegmentedControl/SegmentedControl.slot-recipe';
import { pageHeaderRecipe } from '../components/PageHeader/PageHeader.recipe';
import { albumsPageRecipe } from '../components/AlbumsPage/AlbumsPage.recipe';
import { albumEditorRecipe } from '../components/AlbumEditor/AlbumEditor.recipe';
import { photoAlbumActionsRecipe } from '../components/PhotoAlbumActions/PhotoAlbumActions.recipe';
import { createSystem, defaultBaseConfig, defaultConfig, defineConfig } from '@chakra-ui/react';
import { colors } from './colors';
import { fonts } from './fonts';
import { globalCss } from './global-css';
import { keyframes } from './keyframes';
import { semanticTokens } from './semantic-tokens';
import { buttonRecipe } from '../components/Button/Button.recipe';
import { inputRecipe } from '../components/Input/Input.recipe';
import { textareaRecipe } from '../components/Textarea/Textarea.recipe';
import { placeSearchRecipe } from '../components/PlaceSearch/PlaceSearch.slot-recipe';
import { inspectionChoiceCardsRecipe } from '../components/InspectionChoiceCards/InspectionChoiceCards.slot-recipe';
import { errorToastRecipe } from '../components/ErrorToast/ErrorToast.slot-recipe';
import { storageExplorerRecipe } from '../components/StorageExplorer/StorageExplorer.recipe';
import { inspectionDialogRecipe } from '../components/InspectionDialog/InspectionDialog.recipe';

export const system = createSystem(
  defaultBaseConfig,
  defineConfig({
    cssVarsPrefix: 'stillharbour',
    // Retain the desktop's native form, dialog and photographic canvas behavior.
    preflight: false,
    globalCss,
    theme: {
      breakpoints: { sm: '480px', md: '768px', lg: '1024px', xl: '1280px', '2xl': '1536px' },
      keyframes,
      tokens: {
        spacing: {
          controlY: { value: '7px' },
          1: { value: '0.25rem' },
          1.5: { value: '6px' },
          2: { value: '0.5rem' },
          3: { value: '0.75rem' },
          4: { value: '1rem' },
          5: { value: '1.25rem' },
          6: { value: '1.5rem' },
          8: { value: '2rem' },
        },
        lineHeights: { title: { value: '24px' }, control: { value: '20px' } },
        fontSizes: {
          md: { value: '16px' },
          lg: { value: '18px' },
          xl: { value: '22px' },
          '2xl': { value: '26px' },
          xs: { value: '12px' },
          sm: { value: '13px' },
        },
        zIndex: { popover: { value: 1500 }, modal: { value: 1400 }, tooltip: { value: 1600 } },
        colors,
        fonts,
        radii: { control: { value: '8px' }, panel: { value: '12px' } },
        sizes: {
          appHeader: { value: '64px' },
          control: { value: '36px' },
          full: { value: '100%' },
          sm: { value: '24rem' },
          md: { value: '28rem' },
          lg: { value: '32rem' },
          '2xl': { value: '42rem' },
          24: { value: '6rem' },
          toast: { value: '34rem' },
          64: { value: '16rem' },
          16: { value: '4rem' },
        },
      },
      semanticTokens,
      slotRecipes: {
        dialog: structureDialogRecipe,
        organizationPresets: organizationPresetsRecipe,
        segmentGroup: segmentedControlRecipe,
        field: defaultConfig.theme!.slotRecipes!.field!,
        radioCard: inspectionChoiceCardsRecipe,
        popover: placeSearchRecipe,
        toast: errorToastRecipe,
      },
      recipes: {
        structurePathBuilder: structurePathBuilderRecipe,
        structureFormatter: structureFormatterRecipe,
        pageHeader: pageHeaderRecipe,
        albumsPage: albumsPageRecipe,
        albumEditor: albumEditorRecipe,
        photoAlbumActions: photoAlbumActionsRecipe,
        storageExplorer: storageExplorerRecipe,
        inspectionDialog: inspectionDialogRecipe,
        button: buttonRecipe,
        input: inputRecipe,
        textarea: textareaRecipe,
      },
    },
  }),
);
