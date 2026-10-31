import { fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import { SavedOrganizationPlans } from './SavedOrganizationPlans';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
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
