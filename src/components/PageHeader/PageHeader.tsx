import { Box, useRecipe } from '@chakra-ui/react';
import type { ReactElement } from 'react';
import { pageHeaderRecipe } from './PageHeader.recipe';

export function PageHeader({ children }: { children: ReactElement }) {
  const styles = useRecipe({ recipe: pageHeaderRecipe })();
  return (
    <Box
      asChild
      className="app-page-header"
      css={styles}
      onContextMenu={(event) => {
        if (event.target instanceof Element && event.target.closest('h1')) event.preventDefault();
      }}
    >
      {children}
    </Box>
  );
}
