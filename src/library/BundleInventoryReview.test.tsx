import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type {
  BundleConfirmation,
  BundleInventory,
  BundleInventoryEntry,
  OrganizationRow,
} from '../bindings';
import { libraryData } from '../ipc';
import { BundleInventoryReview } from './BundleInventoryReview';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const row: OrganizationRow = {
  assetId: 'photo',
  rootId: 'root',
  sourcePath: 'Family/<photo>.JPG',
  sourceRevision: 'revision',
  targetRelativePath: 'out/photo.JPG',
  sequence: 1,
  bytes: '100',
  issues: [],
  usedUndated: false,
  companionCandidates: 0,
};
function entry(
  id: string,
  name: string,
  relation: BundleInventoryEntry['relation'] = 'additional',
): BundleInventoryEntry {
  return {
    id,
    name,
    relation,
    kind: 'file',
    bytes: '9007199254740993',
    catalogAssetId: null,
    catalogPresence: null,
    nameRepresentable: true,
    selectable: relation !== 'primary',
    error: null,
  };
}
const inventory: BundleInventory = {
  assetId: 'photo',
  sourceRevision: 'revision',
  catalogRevision: '10',
  fingerprint: 'a'.repeat(64),
  entries: [
    entry('primary', '<photo>.JPG', 'primary'),
    entry('sidecar', '<photo>.xmp', 'same_stem'),
    entry('hidden', '.hidden.bin'),
  ],
  inventoryComplete: true,
  truncated: false,
  indexedLimitExceeded: false,
  unobservedCandidates: [],
  executionAvailable: false,
};
const confirmation: BundleConfirmation = {
  id: 'review-id',
  planId: 'plan',
  assetId: 'photo',
  inventoryFingerprint: inventory.fingerprint,
  fingerprint: 'b'.repeat(64),
  totalBytes: '9007199254741093',
  executionAvailable: false,
  files: [
    {
      entryId: 'primary',
      sourceName: '<photo>.JPG',
      targetRelativePath: 'out/photo.JPG',
      relation: 'primary',
      bytes: '100',
    },
    {
      entryId: 'sidecar',
      sourceName: '<photo>.xmp',
      targetRelativePath: 'out/photo.xmp',
      relation: 'same_stem',
      bytes: '9007199254740993',
    },
  ],
};
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
});
function show() {
  return render(
    <BundleInventoryReview
      planId="plan"
      row={row}
      catalogRevision="10"
      stale={false}
      onClose={vi.fn()}
    />,
  );
}
async function read() {
  fireEvent.click(screen.getByRole('button', { name: 'Read all filenames' }));
  await screen.findByRole('table');
}

it('reads all entries only on request and retains an explicitly reviewed ID-only companion selection', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce(inventory).mockResolvedValueOnce(confirmation);
  show();
  expect(libraryData).not.toHaveBeenCalled();
  await read();
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_bundle_inventory', asset_id: 'photo', source_revision: 'revision' },
    'organization_bundle_inventory',
  );
  expect(screen.getByRole('checkbox', { name: 'Include <photo>.JPG' })).toBeDisabled();
  expect(screen.getByRole('checkbox', { name: 'Include <photo>.xmp' })).toBeChecked();
  expect(screen.getByRole('checkbox', { name: 'Include .hidden.bin' })).not.toBeChecked();
  expect(screen.getByRole('button', { name: 'Save companion review' })).toBeDisabled();
  fireEvent.click(screen.getByRole('checkbox', { name: /I reviewed all filenames/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Save companion review' }));
  expect(await screen.findByRole('status')).toHaveTextContent(
    '2 files · 9,007,199,254,741,093 bytes',
  );
  expect(libraryData).toHaveBeenLastCalledWith(
    {
      kind: 'organization_bundle_confirm',
      input: {
        planId: 'plan',
        assetId: 'photo',
        sourceRevision: 'revision',
        inventoryFingerprint: inventory.fingerprint,
        companionIds: ['sidecar'],
        reviewedAllEntries: true,
      },
    },
    'organization_bundle_confirmation',
  );
  expect(screen.getByRole('button', { name: 'Save companion review' })).toBeDisabled();
  expect(document.querySelector('photo')).toBeNull();
});

it('blocks omitted possible companions, incomplete inventories and responses from a different source', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(inventory)
    .mockResolvedValueOnce({ ...inventory, inventoryComplete: false, truncated: true })
    .mockResolvedValueOnce({ ...inventory, assetId: 'different-photo' });
  show();
  await read();
  fireEvent.click(screen.getByRole('checkbox', { name: 'Include <photo>.xmp' }));
  fireEvent.click(screen.getByRole('checkbox', { name: /I reviewed all filenames/ }));
  expect(screen.getByRole('button', { name: 'Save companion review' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Read filenames again' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('incomplete');
  expect(screen.getByRole('checkbox', { name: /I reviewed all filenames/ })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Save companion review' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Read filenames again' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('source changed');
  expect(screen.getByRole('button', { name: 'Save companion review' })).toBeDisabled();
});

it('pages a bounded complete response without dropping selections or opening source files from names', async () => {
  const large = {
    ...inventory,
    entries: [
      ...inventory.entries.slice(0, 2),
      ...Array.from({ length: 65 }, (_, index) => entry(`other-${index}`, `other-${index}.bin`)),
    ],
  };
  vi.mocked(libraryData).mockResolvedValueOnce(large);
  show();
  await read();
  expect(screen.getByRole('table')).toHaveAccessibleName(/page 1 of 2/);
  expect(screen.queryByRole('checkbox', { name: 'Include other-64.bin' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Next filenames' }));
  fireEvent.click(screen.getByRole('checkbox', { name: 'Include other-64.bin' }));
  fireEvent.click(screen.getByRole('button', { name: 'Previous filenames' }));
  expect(screen.getByRole('checkbox', { name: 'Include <photo>.xmp' })).toBeChecked();
  expect(screen.getByText(/3 files selected for review/)).toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledOnce();
});

it('preserves a failed decision for correction, reloads history separately and ignores late inventory results after close', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(inventory)
    .mockRejectedValueOnce({ message: 'The source changed. Review again.' })
    .mockResolvedValueOnce([confirmation]);
  const view = show();
  await read();
  fireEvent.click(screen.getByRole('checkbox', { name: /I reviewed all filenames/ }));
  fireEvent.click(screen.getByRole('button', { name: 'Save companion review' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('source changed');
  expect(screen.getByRole('checkbox', { name: 'Include <photo>.xmp' })).toBeChecked();
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Load retained companion reviews' }));
  await screen.findByRole('button', { name: /View 2 files/ });
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_bundle_reviews', plan_id: 'plan', asset_id: 'photo' },
    'organization_bundle_reviews',
  );
  let resolve!: (inventory: BundleInventory) => void;
  vi.mocked(libraryData).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Read filenames again' }));
  await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(4));
  view.unmount();
  await act(async () => {
    resolve(inventory);
  });
  expect(screen.queryByRole('table')).not.toBeInTheDocument();
});
