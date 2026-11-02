import { fireEvent, render, screen, waitFor } from '../test/render';
import { expect, it, vi } from 'vitest';
import { KeeperControls } from './KeeperControls';
import { libraryData } from '../ipc';
import type { InspectionFinding } from '../bindings';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const member: InspectionFinding = {
  id: 'member',
  assetId: null,
  rootId: 'root',
  folderId: 'folder',
  name: 'Generated.png',
  displayPath: '/generated/Generated.png',
  bytes: '100',
  reportedAllocatedBytes: null,
  objectId: 'object',
  links: '1',
  isMedia: true,
  inventory: 'present',
  content: 'not_checked',
  checksum: 'not_checked',
  comparisonName: null,
  filesystemModified: null,
  filesystemCreated: null,
  readonly: false,
  baselineId: null,
  decoderPolicy: null,
  pixelSignature: null,
  pixelDigest: null,
  width: null,
  height: null,
  format: null,
  coverage: 'Not checked',
  error: null,
};
it('sends protected folder and explicit keep IDs to the backend and renders blocked review', async () => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind !== 'inspection_keeper_review') throw new Error('Unexpected request');
      return {
        id: 'review',
        input: request.input,
        keepers: ['member'],
        suggestionBlock: null,
        blocks: [{ assetId: 'member', reason: 'fresh_bytes_required' }],
        explanation: 'Fresh complete bytes required',
        mutationAvailable: false,
      };
    });
  render(
    <KeeperControls
      jobId="job"
      groupId="group"
      rows={[member]}
      keep={new Set(['member'])}
      onKeep={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole('checkbox', { name: 'Protect this folder' }));
  fireEvent.click(screen.getByRole('button', { name: 'Retain backend keeper review' }));
  expect(await screen.findByRole('status')).toHaveTextContent('Fresh complete bytes required');
  await waitFor(() =>
    expect(libraryData).toHaveBeenCalledWith(
      {
        kind: 'inspection_keeper_review',
        input: {
          jobId: 'job',
          groupId: 'group',
          order: 'manual',
          manualKeepers: ['member'],
          protectedFolders: ['folder'],
          preferredFolders: [],
        },
      },
      'inspection_keeper_review',
    ),
  );
  expect(screen.getByText(/Quarantine remains unavailable/)).toBeVisible();
});
