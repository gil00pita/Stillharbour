import { expect, it, vi, beforeEach } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '../../../test/render';
import { libraryData } from '../../../ipc';
import type { OrganizationExamplePreview, OrganizationTemplate } from '../../../bindings';
import { StructureExamples } from './StructureExamples';

vi.mock('../../../ipc', async (original) => ({
  ...(await original<typeof import('../../../ipc')>()),
  libraryData: vi.fn(),
}));
const template: OrganizationTemplate = {
  pattern: '{capture:%Y}/{original_stem}.{ext}',
  event: '',
  missingDate: 'undated',
};
function examples(target: string): OrganizationExamplePreview {
  return {
    sampleOnly: true,
    rows: [
      {
        assetId: 'illustrative-0',
        rootId: 'illustrative',
        sourcePath: 'Unsorted/IMG_0421.JPG',
        sourceRevision: 'illustrative',
        sourceComponents: ['Unsorted', 'IMG_0421.JPG'],
        targetRelativePath: target,
        sequence: 1,
        bytes: '0',
        issues: [],
        usedUndated: false,
        companionCandidates: 0,
      },
    ],
  };
}
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
});
it('renders native resolved tree and sample paths with an explicit illustrative label', async () => {
  vi.mocked(libraryData).mockResolvedValue(examples('2026/IMG_0421.JPG'));
  render(<StructureExamples template={template} />);
  expect(await screen.findByText('Example library')).toBeVisible();
  expect(screen.getByText('2026')).toBeVisible();
  expect(screen.getByText(/No library files are inspected/)).toBeVisible();
  fireEvent.click(screen.getByRole('radio', { name: 'Sample paths' }));
  expect(await screen.findByText('2026/IMG_0421.JPG')).toBeVisible();
  expect(libraryData).toHaveBeenCalledWith(
    { kind: 'organization_template_examples', template },
    'organization_template_examples',
  );
});
it('ignores a late example reply after the choices change', async () => {
  let resolve!: (value: OrganizationExamplePreview) => void;
  vi.mocked(libraryData)
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce(examples('2026/0001.JPG'));
  const view = render(<StructureExamples template={template} />);
  await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(1));
  view.rerender(
    <StructureExamples template={{ ...template, pattern: '{capture:%Y}/{sequence:04}.{ext}' }} />,
  );
  await screen.findByText('0001.JPG');
  await act(async () => resolve(examples('2026/OLD.JPG')));
  expect(screen.queryByText('OLD.JPG')).not.toBeInTheDocument();
});
it('keeps invalid naming feedback explicit instead of displaying stale examples', async () => {
  vi.mocked(libraryData).mockRejectedValue(new Error('Unknown template token'));
  render(<StructureExamples template={template} />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Unknown template token');
  expect(screen.queryByText('Example library')).not.toBeInTheDocument();
});
