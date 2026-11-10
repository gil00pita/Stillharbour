import { Box, useRecipe } from '@chakra-ui/react';
import { cloneElement, type ReactElement, type ReactNode } from 'react';
import { pageHeaderRecipe } from './PageHeader.recipe';
import { AppearanceToggle } from '@/components/AppearanceToggle';

export function PageHeader({ children }: { children: ReactElement<{ children?: ReactNode }> }) {
  const styles = useRecipe({ recipe: pageHeaderRecipe })();
  return (
    <Box
      asChild
      className="app-page-header"
      css={[
        styles,
        {
          '&.toolbar': {
            height: 'var(--app-header-height)',
            minHeight: 'var(--app-header-height)',
            maxHeight: 'var(--app-header-height)',
            padding: '0 24px',
            flexWrap: 'nowrap',
            alignItems: 'center',
            gap: '12px',
            justifyContent: 'flex-start',
          },
          '& h1': {
            display: 'block',
            flexShrink: '0',
            margin: '0',
            fontSize: 'var(--stillharbour-font-sizes-lg, 18px)',
            lineHeight: 'var(--stillharbour-line-heights-title, 24px)',
            whiteSpace: 'nowrap',
            userSelect: 'none',
            WebkitUserSelect: 'none',
          },
          '& button': {
            height: 'var(--control-height)',
            minHeight: 'var(--control-height)',
            paddingBlock: '0',
          },
          '& .primary': { margin: '0' },
          '& > .appearance-action:only-child': { marginInlineStart: 'auto' },
          '&:has(.fixed-search) > h1': { flex: '0 0 136px' },
          '& > .header-actions': { marginInlineStart: 'auto' },
          '& > .explorer-views': { marginInlineStart: 'auto' },
          '& > .appearance-action:last-child': { marginInlineStart: '0' },
          '& > :first-child:not(button):not(.fixed-search)': { marginInlineEnd: 'auto' },
          '&:has(.fixed-search) > :first-child': { marginInlineEnd: '0' },
          '& .header-actions': {
            minWidth: '0',
            gap: '8px',
            flexWrap: 'nowrap',
            display: 'flex',
            flex: '0 0 auto',
          },
          '& .fixed-search': { width: '0', flex: '1 1 0%', minWidth: '0' },
          '& .unified-search': { display: 'flex', alignItems: 'center', gap: '8px' },
          "& .unified-search > [data-scope='field']": { flex: '1', minWidth: '0' },
          '& .place-search-chip': {
            flex: '0 1 40%',
            margin: '0',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          },
          '@media (max-width: 760px)': {
            '&.toolbar': { paddingInline: '16px', gap: '8px' },
            '&:has(.fixed-search) > h1': { flexBasis: 'auto' },
            '& .toolbar-action-label': { display: 'none' },
            '& .toolbar-secondary': { width: 'var(--control-height)', paddingInline: '0' },
            "& [aria-label='New album']": { width: 'var(--control-height)', paddingInline: '0' },
          },
          '& > .fixed-search': { width: '0', flex: '1 1 0%' },
        },
      ]}
      onContextMenu={(event) => {
        if (event.target instanceof Element && event.target.closest('h1')) event.preventDefault();
      }}
    >
      {cloneElement(
        children,
        {},
        <>
          {children.props.children}
          <Box display="flex" flexShrink="0" className="appearance-action">
            <AppearanceToggle />
          </Box>
        </>,
      )}
    </Box>
  );
}
