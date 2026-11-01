import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData, pickDirectory } from '../ipc';
import { SavedOrganizationPlans } from './SavedOrganizationPlans';
import type { OrganizationPlanDetail } from '../bindings';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
  pickDirectory: vi.fn(),
}));
const summary = {
  id: 'plan-id',
  createdAt: 'now',
  digest: 'digest',
  state: 'draft' as const,
  destinationPath: 'Dest/<folder>',
  proposed: 1,
  blocked: 0,
  selectedBytes: '42',
};
const emptyDetail: OrganizationPlanDetail = {
  stale: false,
  plan: {
    ...summary,
    policy: 'organization-plan-v1',
    templateReference: null,
    destinationFingerprint: 'dest',
    blockers: ['execution_and_recovery'],
    executionAvailable: false,
    preview: {
      policy: 'preview',
      fingerprint: 'fp',
      catalogRevision: '1',
      template: { pattern: '{original_stem}.{ext}', event: '', missingDate: 'block' },
      rows: [],
      proposed: 0,
      blocked: 0,
      selectedBytes: '0',
      executionAvailable: false,
    },
  },
};
async function openEmptyPlan() {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ plans: [summary], next: null })
    .mockResolvedValueOnce(emptyDetail);
  render(<SavedOrganizationPlans onClose={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button', { name: /Dest\/<folder>/ }));
  await screen.findByRole('heading', { name: 'Draft · now' });
}
beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = false;
    },
  });
  vi.mocked(libraryData).mockReset();
  vi.mocked(pickDirectory).mockReset();
});
it('reopens immutable rows, warns about staleness and shows the backend export without executing', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce({ plans: [summary], next: null })
    .mockResolvedValueOnce({
      stale: true,
      plan: {
        ...summary,
        policy: 'organization-plan-v1',
        templateReference: null,
        destinationFingerprint: 'dest',
        blockers: ['execution_and_recovery'],
        executionAvailable: false,
        preview: {
          policy: 'preview',
          fingerprint: 'fp',
          catalogRevision: '1',
          template: { pattern: '{original_stem}.{ext}', event: '', missingDate: 'block' },
          rows: [
            {
              assetId: 'photo',
              rootId: 'root',
              sourcePath: '<source>.JPG',
              sourceRevision: 'rev',
              targetRelativePath: 'after/photo.JPG',
              sequence: 1,
              bytes: '42',
              issues: [],
              usedUndated: false,
              companionCandidates: 0,
            },
          ],
          proposed: 1,
          blocked: 0,
          selectedBytes: '42',
          executionAvailable: false,
        },
      },
    })
    .mockResolvedValueOnce({
      filename: 'plan.json',
      mediaType: 'application/json',
      text: '{"retained":true}',
    });
  render(<SavedOrganizationPlans onClose={vi.fn()} />);
  fireEvent.click(await screen.findByRole('button', { name: /Dest\/<folder>/ }));
  expect(await screen.findByRole('alert')).toHaveTextContent('catalog changed');
  expect(within(screen.getByRole('table')).getByText('<source>.JPG')).toBeVisible();
  expect(screen.getByText('after/photo.JPG')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'View JSON export' }));
  expect(await screen.findByRole('textbox')).toHaveValue('{"retained":true}');
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_plan_export', plan_id: 'plan-id', format: 'json' },
    'organization_plan_export',
  );
});
it('reports load errors and retries without claiming an empty history', async () => {
  vi.mocked(libraryData)
    .mockRejectedValueOnce({ message: 'Catalog unavailable' })
    .mockResolvedValueOnce({ plans: [], next: null });
  render(<SavedOrganizationPlans onClose={vi.fn()} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog unavailable');
  expect(screen.queryByText(/No saved plans/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Refresh plans' }));
  expect(await screen.findByText(/No saved plans/)).toBeVisible();
});

it('saves a new review file only through picker authority and displays the verified receipt', async () => {
  await openEmptyPlan();
  vi.mocked(pickDirectory).mockResolvedValueOnce({
    id: 'export-session',
    displayPath: 'Selected folder',
    identityAvailable: true,
  });
  vi.mocked(libraryData).mockResolvedValueOnce({
    path: 'Output/<review>.json',
    bytes: '9007199254740993',
    digest: 'content-digest',
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save JSON file…' }));
  expect(await screen.findByRole('status')).toHaveTextContent(
    'Review file saved: Output/<review>.json · 9,007,199,254,740,993 bytes verified.',
  );
  expect(libraryData).toHaveBeenLastCalledWith(
    {
      kind: 'organization_plan_save_export',
      plan_id: 'plan-id',
      format: 'json',
      selection_id: 'export-session',
    },
    'organization_plan_export_saved',
  );
  expect(document.querySelector('review')).toBeNull();
});

it('keeps cancellation and unsupported selections separate from native export success', async () => {
  await openEmptyPlan();
  vi.mocked(pickDirectory).mockResolvedValueOnce(null).mockResolvedValueOnce({
    id: 'unsupported',
    displayPath: 'Unavailable',
    identityAvailable: false,
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save CSV file…' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Save CSV file…' })).toBeEnabled());
  expect(libraryData).toHaveBeenCalledTimes(2);
  fireEvent.click(screen.getByRole('button', { name: 'Save CSV file…' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('verifiable native identity');
  expect(libraryData).toHaveBeenCalledTimes(2);
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
});

it('reports native export conflicts without fabricating a saved file', async () => {
  await openEmptyPlan();
  vi.mocked(pickDirectory).mockResolvedValueOnce({
    id: 'export-session',
    displayPath: 'Selected folder',
    identityAvailable: true,
  });
  vi.mocked(libraryData).mockRejectedValueOnce({
    message: 'Existing file retained. Choose another folder.',
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save JSON file…' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Existing file retained');
  expect(screen.queryByRole('status')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Save JSON file…' })).toBeEnabled();
});
