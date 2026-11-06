import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import type { LibraryStatus } from '../../bindings';
import { defaultQuery } from '../../library/model';
import { CatalogActions } from '@/components/CatalogActions';
vi.mock('@/components/CatalogOrganizer', () => ({
  CatalogOrganizer: ({
    onBrowse,
    onClose,
  }: {
    onBrowse: (query: typeof defaultQuery) => void;
    onClose: () => void;
  }) => (
    <div role="dialog" aria-label="Catalog">
      <button
        onClick={() => {
          onBrowse({ ...defaultQuery, tag: 'Coast' });
          onClose();
        }}
      >
        Browse Coast
      </button>
    </div>
  ),
}));
vi.mock('@/components/SavedOrganizationPlans', () => ({
  SavedOrganizationPlans: ({ onClose }: { onClose: () => void }) => (
    <div role="dialog" aria-label="Saved plans">
      <button onClick={onClose}>Close plans</button>
    </div>
  ),
}));
const status: LibraryStatus = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
it('opens the existing catalog and saved plans, and sends catalog queries back to Photos', () => {
  const onBrowse = vi.fn();
  render(<CatalogActions status={status} onBrowse={onBrowse} onRefresh={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Catalog' }));
  expect(screen.getByRole('dialog', { name: 'Catalog' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Browse Coast' }));
  expect(onBrowse).toHaveBeenCalledWith({ ...defaultQuery, tag: 'Coast' });
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Saved plans' }));
  expect(screen.getByRole('dialog', { name: 'Saved plans' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Close plans' }));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});
