import { Text } from '@chakra-ui/react';
import { useState } from 'react';
import { Button } from '@/components/Button';
import { CatalogOrganizer } from '@/components/CatalogOrganizer';
import { SavedOrganizationPlans } from '@/components/SavedOrganizationPlans';
import type { AssetQuery, LibraryStatus } from '../../bindings';
import { defaultQuery } from '../../library/model';

export function CatalogActions({
  status,
  onBrowse,
  onRefresh,
}: {
  status: LibraryStatus;
  onBrowse: (query: AssetQuery) => void;
  onRefresh: () => Promise<void>;
}) {
  const [open, setOpen] = useState<'catalog' | 'plans' | null>(null);
  return (
    <>
      <Button
        type="button"
        className="toolbar-secondary"
        aria-label="Catalog"
        title="Catalog"
        onClick={() => setOpen('catalog')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M4 5h16v15H4ZM8 3v4m8-4v4M4 10h16M8 14h3m-3 3h7"
            stroke="currentColor"
            strokeWidth="1.5"
          />
        </svg>
        <Text as="span" className="toolbar-action-label">
          Catalog
        </Text>
      </Button>
      <Button
        type="button"
        className="toolbar-secondary"
        aria-label="Saved plans"
        title="Saved plans"
        onClick={() => setOpen('plans')}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 3h12v18l-6-4-6 4Z" stroke="currentColor" strokeWidth="1.5" />
        </svg>
        <Text as="span" className="toolbar-action-label">
          Saved plans
        </Text>
      </Button>
      {open === 'catalog' && (
        <CatalogOrganizer
          roots={status.roots}
          query={defaultQuery}
          selection={null}
          selectionCount={0}
          onBrowse={onBrowse}
          onChanged={onRefresh}
          onClose={() => setOpen(null)}
        />
      )}
      {open === 'plans' && <SavedOrganizationPlans onClose={() => setOpen(null)} />}
    </>
  );
}
