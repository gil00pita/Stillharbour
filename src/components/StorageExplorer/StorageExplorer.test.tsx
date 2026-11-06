import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '../../test/render';
import { StorageExplorer } from '@/components/StorageExplorer';
import { storageFile, storageFiles, storageRoot, storageSummary } from './fixtures';
import { libraryData } from '../../ipc';
import { fixtureAsset } from '../../library/__tests__/fixtures';
import { ageOf, driveSegments, emptyFilters, matches } from './model';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'detail')
        return {
          asset: { ...fixtureAsset('asset'), camera: 'Canon EOS R5' },
          root: storageRoot,
          rawMetadata: null,
          companions: [],
        };
      if (request.kind === 'inspection_folders')
        return { jobId: 'job', parent: storageSummary.total, rows: [], next: null };
      if (request.kind === 'inspection_findings')
        return { jobId: 'job', rows: [storageFile('more')], next: null };
      throw new Error('Unexpected request');
    });
});
function setup(next: string | null = null) {
  return render(
    <StorageExplorer
      jobId="job"
      summary={storageSummary}
      roots={[storageRoot]}
      volumes={[
        {
          id: 'volume',
          rootIds: ['root'],
          totalBytes: '20000000000',
          availableBytes: '5000000000',
          observedAt: '2026-11-05',
          coverage: 'Generated observation',
        },
      ]}
      firstPage={{ jobId: 'job', rows: storageFiles, next }}
    />,
  );
}
it('keeps physical drive totals stable through category filters and uses only the requested colour modes', async () => {
  setup();
  const overview = screen.getByRole('region', { name: 'Storage overview' });
  const filters = screen.getByRole('region', { name: 'Storage filters' });
  expect(filters.compareDocumentPosition(overview) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect([...filters.querySelectorAll('label')].map((label) => label.textContent)).toEqual([
    'Search',
    'File type',
    'Camera',
    'Age · last modified',
    'Minimum size',
  ]);
  expect(overview).toHaveTextContent('15 GB of 20 GB used · 5 GB available');
  expect(
    [...screen.getByLabelText('Colour mode').querySelectorAll('option')].map(
      (option) => option.textContent,
    ),
  ).toEqual(['File type', 'Age', 'Camera']);
  fireEvent.click(within(overview).getByRole('button', { name: /^Videos: / }));
  expect(screen.getByText('1 files · 9.6 GB')).toBeVisible();
  expect(overview).toHaveTextContent('15 GB of 20 GB used · 5 GB available');
  expect(within(overview).getByRole('button', { name: /^Videos: / })).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Remove category filter' }));
  expect(screen.getByText('3 files · 9.6 GB')).toBeVisible();
  await screen.findByRole('option', { name: /Canon EOS R5/ });
});
it('changes tile colours without moving rectangles and drills with keyboard-operable explicit controls', async () => {
  const { container } = setup();
  const geometry = () =>
    [...container.querySelectorAll<HTMLElement>('.storage-tile')].map((tile) => [
      tile.style.left,
      tile.style.top,
      tile.style.width,
      tile.style.height,
    ]);
  const before = geometry();
  const color = container.querySelector<HTMLElement>('.storage-tile')!.style.background;
  fireEvent.change(screen.getByLabelText('Colour mode'), { target: { value: 'Age' } });
  expect(geometry()).toEqual(before);
  expect(container.querySelector<HTMLElement>('.storage-tile')!.style.background).not.toBe(color);
  fireEvent.click(screen.getByRole('button', { name: /^Generated library: / }));
  fireEvent.click(screen.getByRole('button', { name: 'Open folder' }));
  expect(screen.getByRole('button', { name: 'Parent folder' })).toBeEnabled();
  expect(screen.getByRole('button', { name: /^Trips: / })).toBeVisible();
  fireEvent.doubleClick(screen.getByRole('button', { name: /^Trips: / }));
  expect(screen.getByRole('navigation', { name: 'Storage folders' })).toHaveTextContent('Trips');
  await screen.findByRole('option', { name: /Canon EOS R5/ });
});
it('keeps empty files accessible, respects protection, and makes review exclusion a simulation', async () => {
  setup();
  fireEvent.click(screen.getByRole('button', { name: 'List' }));
  const list = screen.getByRole('region', { name: 'Inspected files' });
  expect(list).toHaveTextContent('0 B');
  fireEvent.click(within(list).getByRole('button', { name: 'empty.txt' }));
  expect(screen.getByRole('button', { name: 'Add to review tray' })).toBeDisabled();
  fireEvent.click(within(list).getByRole('button', { name: 'notes.txt' }));
  fireEvent.click(screen.getByRole('button', { name: 'Add to review tray' }));
  fireEvent.click(screen.getByRole('button', { name: 'Preview without these files' }));
  expect(screen.getByRole('status')).toHaveTextContent('not guaranteed reclaimed disk space');
  expect(within(list).queryByRole('button', { name: 'notes.txt' })).not.toBeInTheDocument();
  expect(screen.getByRole('region', { name: 'Storage overview' })).toHaveTextContent(
    '15 GB of 20 GB used',
  );
  fireEvent.click(screen.getByRole('button', { name: 'End simulation' }));
  expect(within(list).getByRole('button', { name: 'notes.txt' })).toBeVisible();
  await screen.findByRole('option', { name: /Canon EOS R5/ });
});
it('combines path search, size, age and recorded camera, and labels empty results', async () => {
  setup();
  await screen.findByRole('option', { name: /Canon EOS R5/ });
  fireEvent.change(screen.getByLabelText('Camera'), { target: { value: 'Canon EOS R5' } });
  fireEvent.change(screen.getByLabelText('Search'), { target: { value: 'Trips' } });
  fireEvent.change(screen.getByLabelText('Minimum size'), { target: { value: '1000000000' } });
  expect(screen.getByText('1 files · 9.6 GB')).toBeVisible();
  fireEvent.change(screen.getByLabelText('Search'), { target: { value: 'absent' } });
  expect(screen.getByRole('status')).toHaveTextContent('No files match');
  fireEvent.click(screen.getByRole('button', { name: 'Clear storage filters' }));
  expect(screen.getByText('3 files · 9.6 GB')).toBeVisible();
});
it('discloses partial paging, loads more observations, and preserves failures as unknown cameras', async () => {
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind === 'detail') throw new Error('Root offline');
    if (request.kind === 'inspection_findings')
      return { jobId: 'job', rows: [storageFile('more')], next: null };
    if (request.kind === 'inspection_folders')
      return { jobId: 'job', parent: storageSummary.total, rows: [], next: null };
    throw new Error('Unexpected');
  });
  setup('cursor');
  expect(screen.getByText(/Filters and file-type totals cover loaded/)).toBeVisible();
  await screen.findByRole('button', { name: 'Retry camera metadata' });
  fireEvent.click(screen.getByRole('button', { name: 'Load more file observations' }));
  await waitFor(() => expect(screen.getByText('4 files · 9.6 GB')).toBeVisible());
  expect(screen.getByLabelText('Camera')).toHaveTextContent('Unknown camera · 1 files');
  expect(screen.getByLabelText('Camera')).not.toHaveTextContent('Canon');
});
it('never invents free space from a capacity or a partial logical scan, and dates use the clock', () => {
  expect(driveSegments('1000', null)).toBeNull();
  expect(driveSegments(null, '500')).toBeNull();
  expect(driveSegments('1000', '1001')).toBeNull();
  expect(driveSegments('1000', '200')).toEqual({ capacity: '1000', used: '800', free: '200' });
  const now = Date.parse('2030-04-30T00:00:00Z');
  expect(ageOf(storageFile('recent', { filesystemModified: '2030-04-20T00:00:00Z' }), now)).toBe(
    'Last 30 days',
  );
  expect(ageOf(storageFile('future', { filesystemModified: '2031-01-01T00:00:00Z' }), now)).toBe(
    'Unknown date',
  );
  expect(matches(storageFile('Canon-file'), { ...emptyFilters, camera: 'Known camera' }, {})).toBe(
    false,
  );
});
