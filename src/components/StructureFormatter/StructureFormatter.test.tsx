import { beforeEach, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import type { LibraryOrganizationPreview, LibraryStatus } from '../../bindings';
import { library, libraryData } from '../../ipc';
import { StructureFormatter } from '@/components/StructureFormatter';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
vi.mock('@/components/SavedOrganizationPlans', () => ({
  SavedOrganizationPlans: () => <section aria-label="Saved organization plans" />,
}));
const status = { revision: '12', roots: [] } as unknown as LibraryStatus;
function report(patch: Partial<LibraryOrganizationPreview> = {}): LibraryOrganizationPreview {
  return {
    previewId: 'preview-1',
    catalogRevision: '12',
    total: 501,
    processed: 0,
    proposed: 0,
    blocked: 0,
    complete: false,
    offset: 0,
    rows: [],
    executionAvailable: false,
    ...patch,
  };
}
function complete(offset = 0): LibraryOrganizationPreview {
  return report({
    processed: 501,
    proposed: 500,
    blocked: 1,
    complete: true,
    offset,
    rows: [
      {
        assetId: `photo-${offset}`,
        rootId: 'root',
        sourcePath: `source-${offset}.JPG`,
        sourceRevision: 'rev',
        sourceComponents: [],
        targetRelativePath: `2024/photo-${offset}.JPG`,
        sequence: offset + 1,
        bytes: '42',
        issues: offset === 480 ? ['source_unavailable'] : [],
        usedUndated: false,
        companionCandidates: 0,
      },
    ],
  });
}
beforeEach(() => {
  vi.mocked(library)
    .mockReset()
    .mockResolvedValue({ kind: 'organization_protection_status', data: { blockers: [] } });
  vi.mocked(libraryData).mockReset();
});
it('opens directly on rules and whole-library scope without selecting photos', () => {
  render(<StructureFormatter status={status} />);
  expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Structure Formater');
  expect(screen.getByRole('heading', { name: 'Entire library' })).toBeVisible();
  expect(screen.getByRole('textbox', { name: 'Path template' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Preview restructure' })).toBeEnabled();
  expect(screen.queryByRole('button', { name: 'Choose photos' })).not.toBeInTheDocument();
  expect(screen.queryByRole('textbox', { name: 'Event name' })).not.toBeInTheDocument();
  expect(screen.queryByRole('combobox', { name: 'Preset' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Run restructure' })).toBeDisabled();
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Saved plans' }));
  expect(screen.getByRole('region', { name: 'Saved organization plans' })).toBeVisible();
});
it('processes all batches before presenting the global namespace and pages by native handle', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(report())
    .mockResolvedValueOnce(report({ processed: 240 }))
    .mockResolvedValueOnce(report({ processed: 480 }))
    .mockResolvedValueOnce(complete())
    .mockResolvedValueOnce(complete(240));
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('radio', { name: 'Date & time' }));
  await waitFor(() =>
    expect(screen.getByRole('textbox', { name: 'Path template' })).toHaveValue(
      '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
    ),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  await screen.findByRole('region', { name: 'Proposed library structure' });
  expect(libraryData).toHaveBeenNthCalledWith(
    1,
    {
      kind: 'organization_library_start',
      template: expect.objectContaining({
        event: '',
        pattern: '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
      }),
    },
    'organization_library_preview',
  );
  expect(
    vi
      .mocked(libraryData)
      .mock.calls.filter(([request]) => request.kind === 'organization_library_advance'),
  ).toHaveLength(3);
  expect(screen.getByRole('status')).toHaveTextContent(
    '501 library items · 500 proposed · 1 blocked',
  );
  expect(screen.getByText('source-0.JPG')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
  await screen.findByText('source-240.JPG');
  expect(libraryData).toHaveBeenLastCalledWith(
    { kind: 'organization_library_page', preview_id: 'preview-1', offset: 240 },
    'organization_library_preview',
  );
  expect(screen.queryByText('source-0.JPG')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Run restructure' })).toBeDisabled();
});
it('cancels in-flight work, releases its native scope and ignores its late result', async () => {
  let resolve!: (result: LibraryOrganizationPreview) => void;
  vi.mocked(libraryData)
    .mockResolvedValueOnce(report())
    .mockReturnValueOnce(
      new Promise((done) => {
        resolve = done;
      }),
    );
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  await waitFor(() => expect(libraryData).toHaveBeenCalledTimes(2));
  expect(screen.getByRole('status')).toHaveTextContent('0 of 501');
  fireEvent.click(screen.getByRole('button', { name: 'Cancel preview' }));
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_library_discard',
    preview_id: 'preview-1',
  });
  await act(async () => resolve(complete()));
  expect(
    screen.queryByRole('region', { name: 'Proposed library structure' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Preview restructure' })).toBeEnabled();
});
it('cleans up a preview that finishes starting after leaving the page', async () => {
  let resolve!: (result: LibraryOrganizationPreview) => void;
  vi.mocked(libraryData).mockReturnValueOnce(
    new Promise((done) => {
      resolve = done;
    }),
  );
  const view = render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  view.unmount();
  await act(async () => resolve(report()));
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_library_discard',
    preview_id: 'preview-1',
  });
  expect(libraryData).toHaveBeenCalledTimes(1);
});
it('invalidates a completed preview when rules change or catalog metadata changes', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce(complete());
  const view = render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  await screen.findByRole('region', { name: 'Proposed library structure' });
  view.rerender(<StructureFormatter status={{ ...status, revision: '13' }} />);
  expect(screen.getAllByRole('status')[0]).toHaveTextContent('catalog has changed');
  expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  await waitFor(() => expect(screen.getByRole('radio', { name: 'By month' })).toBeEnabled());
  fireEvent.click(screen.getByRole('radio', { name: 'By month' }));
  await waitFor(() =>
    expect(
      screen.queryByRole('region', { name: 'Proposed library structure' }),
    ).not.toBeInTheDocument(),
  );
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_library_discard',
    preview_id: 'preview-1',
  });
});
it('reports native preview failures without displaying a partial result', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce(report())
    .mockRejectedValueOnce(new Error('Catalog changed during preview'));
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog changed during preview');
  expect(
    screen.queryByRole('region', { name: 'Proposed library structure' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Preview restructure' })).toBeEnabled();
});
it('shows an explicit empty library and no pagination', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce(report({ total: 0, complete: true }));
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  expect(await screen.findByText(/No indexed photos or videos yet/)).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Next page' })).not.toBeInTheDocument();
  expect(libraryData).toHaveBeenCalledTimes(1);
});
