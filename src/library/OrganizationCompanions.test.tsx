import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import type { OrganizationCompanionReview } from '../bindings';
import { libraryData } from '../ipc';
import { OrganizationCompanions } from './OrganizationCompanions';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));

function response(): OrganizationCompanionReview {
  return {
    assetId: 'photo-id',
    catalogRevision: '1',
    fingerprint: 'fixture',
    candidates: [
      {
        assetId: 'sidecar-id',
        sourcePath: 'photo.JPG.<script>.xmp',
        sourceRevision: 'revision',
        bytes: '9007199254740993',
        presence: 'excluded',
        rootAvailability: 'offline',
        isMedia: false,
        filenameSidecar: true,
      },
    ],
    truncated: false,
    inventoryComplete: false,
    executionAvailable: false,
  };
}

beforeEach(() => {
  vi.mocked(libraryData).mockReset().mockResolvedValue(response());
});

function setup() {
  const props = { assetId: 'photo-id', previewRevision: '1', revision: '1' };
  return { ...render(<OrganizationCompanions {...props} />), props };
}

it('loads only on explicit review, sends an ID, and discloses candidate states without confirming a bundle', async () => {
  setup();
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Review indexed companions' }));
  expect(await screen.findByText('photo.JPG.<script>.xmp')).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'organization_companions', asset_id: 'photo-id' },
    'organization_companions',
  );
  expect(screen.getByText('Catalog state: excluded · Library: offline')).toBeVisible();
  expect(screen.getByText(/9,007,199,254,740,993 bytes/)).toBeVisible();
  expect(screen.getByText(/not confirmed bundles/)).toBeVisible();
  expect(document.querySelector('script')).toBeNull();
});

it('shows empty and truncated observations without implying a complete inventory', async () => {
  const value = response();
  value.candidates = [];
  vi.mocked(libraryData).mockResolvedValueOnce(value);
  setup();
  fireEvent.click(screen.getByRole('button'));
  expect(await screen.findByText(/does not prove there are no companions/)).toBeVisible();
  const truncated = response();
  truncated.truncated = true;
  vi.mocked(libraryData).mockResolvedValueOnce(truncated);
  fireEvent.click(screen.getByRole('button'));
  expect(await screen.findByRole('status')).toHaveTextContent('Showing 1 candidates; more indexed');
});

it('exposes errors and retry, and marks catalog changes before and after a response as stale', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Catalog unavailable'));
  const { rerender, props } = setup();
  fireEvent.click(screen.getByRole('button'));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  let resolve!: (value: OrganizationCompanionReview) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  fireEvent.click(screen.getByRole('button'));
  expect(screen.getByRole('button')).toBeDisabled();
  rerender(<OrganizationCompanions {...props} revision="2" />);
  resolve(response());
  expect(await screen.findByRole('alert')).toHaveTextContent('Build a fresh preview');
  const newer = response();
  newer.catalogRevision = '2';
  vi.mocked(libraryData).mockResolvedValueOnce(newer);
  fireEvent.click(screen.getByRole('button'));
  await waitFor(() => expect(screen.getByRole('button')).toBeEnabled());
  expect(screen.getByRole('alert')).toHaveTextContent('Build a fresh preview');
});

it('discards an in-flight response when the preview row unmounts', async () => {
  let resolve!: (value: OrganizationCompanionReview) => void;
  vi.mocked(libraryData).mockImplementationOnce(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const { unmount } = setup();
  fireEvent.click(screen.getByRole('button'));
  unmount();
  resolve(response());
  await waitFor(() => expect(screen.queryByText('photo.JPG.<script>.xmp')).toBeNull());
});
