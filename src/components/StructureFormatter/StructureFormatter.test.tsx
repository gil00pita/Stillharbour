import { beforeEach, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import type {
  LibraryOrganizationPreview,
  LibraryRequest,
  LibraryStatus,
  OrganizationExamplePreview,
  OrganizationPreviewQuery,
} from '../../bindings';
import { library } from '../../ipc';
import { StructureFormatter } from '@/components/StructureFormatter';

const nativeData = vi.hoisted(() => vi.fn());
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  library: vi.fn(),
  libraryData: nativeData,
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
    matching: 501,
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
    summary: {
      moves: 500,
      folders: 12,
      renames: 12,
      unchanged: 0,
      attention: 2,
      missingDates: 1,
      skipped: 0,
    },
    rows: [
      {
        assetId: `photo-${offset}`,
        rootId: 'root',
        sourcePath: `source-${offset}.JPG`,
        sourceRevision: 'rev',
        sourceComponents: [`source-${offset}.JPG`],
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
let start: LibraryOrganizationPreview | Promise<LibraryOrganizationPreview>;
let advances: (LibraryOrganizationPreview | Promise<LibraryOrganizationPreview> | Error)[];
let queryPage: (
  offset: number,
  query: OrganizationPreviewQuery,
) => LibraryOrganizationPreview | Promise<LibraryOrganizationPreview>;
let validation: LibraryOrganizationPreview | Error;
const examples: OrganizationExamplePreview = { sampleOnly: true, rows: complete().rows };
beforeEach(() => {
  vi.mocked(library)
    .mockReset()
    .mockResolvedValue({ kind: 'organization_protection_status', data: { blockers: [] } });
  start = complete();
  advances = [];
  validation = complete();
  queryPage = (offset) => complete(offset);
  nativeData.mockReset().mockImplementation((request: LibraryRequest) => {
    switch (request.kind) {
      case 'organization_template_examples':
        return Promise.resolve(examples);
      case 'organization_library_start':
        return Promise.resolve(start);
      case 'organization_library_advance': {
        const value = advances.shift();
        return value instanceof Error ? Promise.reject(value) : Promise.resolve(value);
      }
      case 'organization_library_query':
        return Promise.resolve(queryPage(request.offset, request.query));
      case 'organization_library_page':
        return validation instanceof Error
          ? Promise.reject(validation)
          : Promise.resolve(validation);
      default:
        return Promise.reject(new Error(`Unexpected request ${request.kind}`));
    }
  });
});
function requests(kind: string) {
  return nativeData.mock.calls
    .map(([request]) => request as LibraryRequest)
    .filter((request) => request.kind === kind);
}
function rawRule() {
  return screen.getByLabelText('Path template') as HTMLInputElement;
}
async function review() {
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  await screen.findByRole('heading', { name: 'Review your changes' });
}

it('opens on whole-library rules, working naming choices and labelled examples without selecting photos', async () => {
  render(<StructureFormatter status={status} />);
  expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Structure Formater');
  expect(screen.getByRole('heading', { name: 'Entire library' })).toBeVisible();
  expect(screen.getByRole('radio', { name: 'Original' })).toBeChecked();
  expect(screen.getByRole('radio', { name: 'Skip photo' })).toBeEnabled();
  expect(screen.queryByRole('button', { name: 'Choose photos' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Run restructure' })).toBeDisabled();
  await screen.findByText('Example library');
  expect(requests('organization_library_start')).toHaveLength(0);
  fireEvent.click(screen.getByRole('button', { name: 'Saved plans' }));
  expect(screen.getByRole('region', { name: 'Saved organization plans' })).toBeVisible();
});
it('keeps filenames independent from folder categories and submits missing-date/case policies', async () => {
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('radio', { name: 'Sequence number' }));
  await waitFor(() => expect(rawRule().value).toContain('{sequence:04}.{ext}'));
  fireEvent.click(screen.getByRole('radio', { name: 'By location' }));
  fireEvent.click(await screen.findByRole('radio', { name: 'Country → City' }));
  await waitFor(() => expect(rawRule()).toHaveValue('{country}/{city}/{sequence:04}.{ext}'));
  fireEvent.click(screen.getByRole('radio', { name: 'Skip photo' }));
  fireEvent.click(screen.getByRole('radio', { name: 'UPPERCASE' }));
  await waitFor(() =>
    expect(requests('organization_template_examples').at(-1)).toEqual(
      expect.objectContaining({
        template: expect.objectContaining({ missingDate: 'skip', filenameStemCase: 'uppercase' }),
      }),
    ),
  );
  await review();
  expect(requests('organization_library_start')).toEqual([
    expect.objectContaining({
      template: expect.objectContaining({
        pattern: '{country}/{city}/{sequence:04}.{ext}',
        missingDate: 'skip',
        filenameStemCase: 'uppercase',
      }),
    }),
  ]);
});
it('processes every native batch before opening review, then pages by the same native handle', async () => {
  start = report();
  advances = [report({ processed: 240 }), report({ processed: 480 }), complete()];
  render(<StructureFormatter status={status} />);
  await review();
  expect(requests('organization_library_advance')).toHaveLength(3);
  expect(screen.getByRole('status')).toHaveTextContent(
    '501 library items · 500 proposed · 1 blocked',
  );
  expect(screen.getByRole('region', { name: 'Proposed changes' })).toHaveTextContent('500');
  fireEvent.click(screen.getByRole('button', { name: 'Next page' }));
  await screen.findAllByText('source-240.JPG');
  expect(requests('organization_library_query')).toEqual([
    expect.objectContaining({
      preview_id: 'preview-1',
      offset: 240,
      query: { search: '', filter: 'all' },
    }),
  ]);
  expect(screen.queryByText('source-0.JPG')).not.toBeInTheDocument();
});
it('cancels in-flight work, releases native scope and ignores late results', async () => {
  let resolve!: (value: LibraryOrganizationPreview) => void;
  start = report();
  advances = [
    new Promise((done) => {
      resolve = done;
    }),
  ];
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  await waitFor(() => expect(requests('organization_library_advance')).toHaveLength(1));
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
  let resolve!: (value: LibraryOrganizationPreview) => void;
  start = new Promise((done) => {
    resolve = done;
  });
  const view = render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  view.unmount();
  await act(async () => resolve(report()));
  expect(library).toHaveBeenCalledWith({
    kind: 'organization_library_discard',
    preview_id: 'preview-1',
  });
  expect(requests('organization_library_advance')).toHaveLength(0);
});
it('blocks stale review and invalidates the result after returning to edit rules', async () => {
  const view = render(<StructureFormatter status={status} />);
  await review();
  view.rerender(<StructureFormatter status={{ ...status, revision: '13' }} />);
  expect(screen.getAllByRole('status')[0]).toHaveTextContent('catalog has changed');
  expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Simulate plan' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Edit structure' }));
  fireEvent.click(screen.getByRole('radio', { name: 'By month' }));
  await waitFor(() =>
    expect(library).toHaveBeenCalledWith({
      kind: 'organization_library_discard',
      preview_id: 'preview-1',
    }),
  );
  expect(
    screen.queryByRole('region', { name: 'Proposed library structure' }),
  ).not.toBeInTheDocument();
});
it('reports native errors without displaying a partial result', async () => {
  start = report();
  advances = [new Error('Catalog changed during preview')];
  render(<StructureFormatter status={status} />);
  fireEvent.click(screen.getByRole('button', { name: 'Preview restructure' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Catalog changed during preview');
  expect(
    screen.queryByRole('region', { name: 'Proposed library structure' }),
  ).not.toBeInTheDocument();
});
it('shows an explicit empty library and disables simulation', async () => {
  start = {
    ...complete(),
    total: 0,
    processed: 0,
    matching: 0,
    proposed: 0,
    blocked: 0,
    rows: [],
    summary: {
      moves: 0,
      folders: 0,
      renames: 0,
      unchanged: 0,
      attention: 0,
      missingDates: 0,
      skipped: 0,
    },
  };
  render(<StructureFormatter status={status} />);
  await review();
  expect(screen.getByText(/No indexed photos or videos yet/)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Simulate plan' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Next page' })).toBeDisabled();
});
it('searches the complete preview with attention and missing-date filters', async () => {
  queryPage = (offset) => ({
    ...complete(offset),
    matching: 1,
    rows: [
      {
        ...complete().rows[0]!,
        assetId: 'late',
        sourcePath: 'outside-first-page.JPG',
        sourceComponents: ['outside-first-page.JPG'],
        captureMissing: true,
        issues: ['source_unavailable'],
      },
    ],
    summary: complete().summary!,
  });
  render(<StructureFormatter status={status} />);
  await review();
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search preview' }), {
    target: { value: 'outside' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Needs attention' }));
  await screen.findAllByText('outside-first-page.JPG');
  expect(requests('organization_library_query').at(-1)).toEqual(
    expect.objectContaining({ offset: 0, query: { search: 'outside', filter: 'needs_attention' } }),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Show affected' }));
  await waitFor(() =>
    expect(requests('organization_library_query').at(-1)).toEqual(
      expect.objectContaining({ query: { search: 'outside', filter: 'missing_dates' } }),
    ),
  );
  expect(screen.getByRole('region', { name: 'Proposed changes' })).toHaveTextContent('500');
});
it('ignores a late search response after a newer filter request', async () => {
  let resolve!: (value: LibraryOrganizationPreview) => void;
  queryPage = (_offset, query) =>
    query.search === 'first'
      ? new Promise((done) => {
          resolve = done;
        })
      : { ...complete(), matching: 0, rows: [] };
  render(<StructureFormatter status={status} />);
  await review();
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search preview' }), {
    target: { value: 'first' },
  });
  await waitFor(() => expect(requests('organization_library_query')).toHaveLength(1));
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search preview' }), {
    target: { value: 'second' },
  });
  await screen.findByText(
    'No items match this search and filter. Try All photos or clear the search.',
  );
  await act(async () => resolve(complete(240)));
  expect(screen.queryByText('source-240.JPG')).not.toBeInTheDocument();
});
it('saves and reloads session rules across navigation without leaking into another catalog', async () => {
  const catalog = `session-${crypto.randomUUID()}`;
  const view = render(<StructureFormatter status={status} catalogId={catalog} />);
  fireEvent.click(screen.getByRole('radio', { name: 'Sequence number' }));
  fireEvent.click(screen.getByRole('button', { name: 'Save preset' }));
  fireEvent.change(await screen.findByRole('textbox', { name: 'Preset name' }), {
    target: { value: 'My archive' },
  });
  fireEvent.click(screen.getAllByRole('button', { name: 'Save preset' }).at(-1)!);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  view.unmount();
  const other = render(<StructureFormatter status={status} catalogId={`${catalog}-other`} />);
  fireEvent.click(screen.getByRole('button', { name: 'Saved presets' }));
  expect(await screen.findByText(/No session presets yet/)).toBeVisible();
  other.unmount();
  render(<StructureFormatter status={status} catalogId={catalog} />);
  fireEvent.click(screen.getByRole('button', { name: 'Saved presets' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Load preset My archive' }));
  await waitFor(() => expect(screen.getByRole('radio', { name: 'Sequence number' })).toBeChecked());
});
it('confirms a clearly labelled simulation with fresh native validation and no execution request', async () => {
  render(<StructureFormatter status={status} />);
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Simulate plan' }));
  expect(await screen.findByRole('dialog', { name: 'Simulate this plan?' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Keep reviewing' }));
  expect(requests('organization_library_page')).toHaveLength(0);
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  fireEvent.click(screen.getByRole('button', { name: 'Simulate plan' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Run simulation' }));
  expect(await screen.findByRole('heading', { name: 'Simulation complete' })).toHaveFocus();
  expect(
    screen.getByText('No files were moved or renamed. Your real library is unchanged.'),
  ).toBeVisible();
  expect(requests('organization_library_page')).toHaveLength(1);
  expect(
    nativeData.mock.calls.every(([request]) =>
      [
        'organization_template_examples',
        'organization_library_start',
        'organization_library_page',
      ].includes((request as LibraryRequest).kind),
    ),
  ).toBe(true);
  expect(screen.getByRole('button', { name: 'Run restructure' })).toBeDisabled();
});
it('refuses a simulated completion when native revalidation fails', async () => {
  validation = new Error('This whole-library preview expired');
  render(<StructureFormatter status={status} />);
  await review();
  fireEvent.click(screen.getByRole('button', { name: 'Simulate plan' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Run simulation' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('preview expired');
  expect(screen.queryByRole('heading', { name: 'Simulation complete' })).not.toBeInTheDocument();
});

it('hides prior rows and disables simulation when a filtered query fails', async () => {
  queryPage = () => {
    throw new Error('The preview query failed');
  };
  render(<StructureFormatter status={status} />);
  await review();
  fireEvent.change(screen.getByRole('searchbox', { name: 'Search preview' }), {
    target: { value: 'new query' },
  });
  expect(await screen.findByRole('alert')).toHaveTextContent('query failed');
  expect(screen.getByText(/Results are unavailable because the latest query failed/)).toBeVisible();
  expect(screen.queryByText('source-0.JPG')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Simulate plan' })).toBeDisabled();
});
