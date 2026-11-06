import { fireEvent, render, screen, waitFor, within } from '../../test/render';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from '@/components/App';
import type { AssetQuery } from '../../bindings';
import { mapPhotoQuery, type MapBrowseState } from '../../map/state';
import { fixturePreferences } from '../../library/__tests__/fixtures';
import { getStatus, library, libraryData, pickDirectory, getPreferences } from '../../ipc';

vi.mock('@/components/MapScreen', () => ({
  MapScreen: ({
    state,
    onShowLibrary,
  }: {
    state: MapBrowseState;
    onShowLibrary: (query: AssetQuery, id: string | null) => void;
  }) => <button onClick={() => onShowLibrary(mapPhotoQuery(state), null)}>Show in photos</button>,
}));
vi.mock('@/components/LibraryScreen', () => ({
  LibraryScreen: ({
    initialQuery,
    onReturnToMap,
  }: {
    initialQuery?: AssetQuery;
    onReturnToMap?: () => void;
  }) => (
    <div>
      <output aria-label="Photo query">
        {initialQuery?.mapBounds ? 'Map area' : 'All photos'}
      </output>
      {onReturnToMap && <button onClick={onReturnToMap}>Return to map</button>}
    </div>
  ),
}));
vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  getStatus: vi.fn(),
  getPreferences: vi.fn(),
  pickDirectory: vi.fn(),
  library: vi.fn(),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(getPreferences).mockReset().mockResolvedValue(fixturePreferences());
  vi.mocked(getStatus).mockReset().mockResolvedValue({
    name: 'Stillharbour',
    version: '0.1.0',
    milestone: 'M1',
    sqliteVersion: '3.54.1',
    catalogId: 'fixture',
    catalogSchema: 2,
    originalMutationsEnabled: false,
  });
  vi.mocked(libraryData).mockReset().mockResolvedValue({
    roots: [],
    jobs: [],
    revision: '0',
    previewGeneration: '0',
    metadataAvailable: true,
    previewAvailable: true,
  });
  vi.mocked(library).mockReset().mockResolvedValue({ kind: 'done' });
  vi.mocked(pickDirectory).mockReset();
});
async function ready() {
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Choose a folder' })).toBeEnabled(),
  );
}
describe('native library onboarding', () => {
  it('shows a native failure and retries without presenting a working library', async () => {
    vi.mocked(getStatus).mockRejectedValueOnce(new Error('Desktop connection unavailable.'));
    render(<App />);
    const toast = await screen.findByRole('alert');
    expect(toast).toHaveTextContent('Desktop connection unavailable.');
    expect(toast).toHaveAttribute('data-placement', 'bottom');
    expect(document.querySelector('.global-error')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Choose a folder' })).toBeDisabled();
    fireEvent.click(screen.getByRole('button', { name: 'Reconnect' }));
    await ready();
  });
  it('replaces the welcome with the selected form and waits for explicit Save', async () => {
    vi.mocked(pickDirectory)
      .mockResolvedValueOnce({
        id: 'native-id',
        displayPath: '/fixture/港\nphoto',
        identityAvailable: true,
      })
      .mockResolvedValueOnce(null);
    render(<App />);
    await ready();
    expect(screen.getByRole('heading', { name: 'A home for the moments you keep.' })).toBeVisible();
    expect(document.querySelector('.folder-mark')).toBeVisible();
    expect(document.querySelector('.empty-library-art')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
    await screen.findByText('Selected folder');
    await waitFor(() =>
      expect(screen.getByRole('textbox', { name: 'Library name' })).toHaveFocus(),
    );
    expect(screen.getByText('Not scanned')).toBeVisible();
    expect(library).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Choose a folder' })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'A home for the moments you keep.' }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText('Local by design.')).not.toBeInTheDocument();
    const version = screen.getByLabelText('Application version');
    expect(version).toHaveTextContent('0.1.0');
    expect(version.closest('.brand')).toHaveTextContent('Stillharbour 0.1.0');
    expect(version.closest('.sidebar-footer')).toBeNull();
    expect(screen.getByText('/fixture/港 photo', { exact: false })).toBeVisible();
    fireEvent.click(screen.getByRole('button', { name: 'Save & Start scanning' }));
    await waitFor(() =>
      expect(library).toHaveBeenCalledWith(
        expect.objectContaining({
          kind: 'save_and_scan',
          selection_id: 'native-id',
          mode: 'managed',
        }),
      ),
    );
    expect(vi.mocked(library).mock.calls[0]?.[0]).not.toHaveProperty('path');
  });
  it('retains the selected folder and reports a failed save without fake success', async () => {
    vi.mocked(pickDirectory).mockResolvedValue({
      id: 'selected',
      displayPath: 'Generated test folder',
      identityAvailable: true,
    });
    vi.mocked(library).mockRejectedValue({
      code: 'CONFLICT',
      message: 'This folder overlaps an existing library.',
    });
    render(<App />);
    await ready();
    fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Save & Start scanning' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'This folder overlaps an existing library.',
    );
    expect(screen.getByText('Not scanned')).toBeVisible();
    expect(screen.getByRole('button', { name: 'Save & Start scanning' })).toBeEnabled();
  });
  it('keeps identity-unsupported folders out of the scanner', async () => {
    vi.mocked(pickDirectory).mockResolvedValue({
      id: 'unsupported',
      displayPath: 'Unverified volume',
      identityAvailable: false,
    });
    render(<App />);
    await ready();
    fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
    expect(await screen.findByRole('button', { name: 'Save & Start scanning' })).toBeDisabled();
    expect(screen.getByRole('alert')).toHaveTextContent('Scanning is unavailable for this folder');
    expect(screen.getByRole('alert')).toHaveTextContent('ordinary local NTFS');
    expect(
      screen.getByRole('button', { name: 'Save & Start scanning' }),
    ).toHaveAccessibleDescription(/cannot verify folder identity/);
    expect(library).not.toHaveBeenCalled();
  });
});

it('returns to the folder chooser when the selected form is canceled', async () => {
  vi.mocked(pickDirectory).mockResolvedValue(selectedDirectory());
  render(<App />);
  await ready();
  fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Cancel' }));
  expect(screen.getByRole('button', { name: 'Choose a folder' })).toBeVisible();
  expect(screen.queryByText('Selected folder')).not.toBeInTheDocument();
});
function selectedDirectory() {
  return { id: 'generated', displayPath: 'Generated library', identityAvailable: true };
}

it('offers Settings before adding a folder without starting a scan', async () => {
  render(<App />);
  await ready();
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  expect(await screen.findByRole('heading', { name: 'Settings' })).toHaveFocus();
  expect(screen.getByRole('heading', { name: 'Library locations' })).toBeVisible();
  expect(screen.getByRole('button', { name: 'Settings' }).closest('footer')).toHaveClass(
    'sidebar-footer',
  );
  expect(library).not.toHaveBeenCalled();
});
it('keeps failed settings explicit and retries without silently using defaults', async () => {
  vi.mocked(getPreferences).mockRejectedValueOnce(new Error('Saved settings could not be read'));
  render(<App />);
  expect(await screen.findByRole('alert')).toHaveTextContent('Saved settings could not be read');
  expect(screen.queryByRole('button', { name: 'Choose a folder' })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Retry settings' }));
  await ready();
});

it('opens a full Jobs page above Settings and returns to the retained page', async () => {
  render(<App />);
  await ready();
  const jobs = screen.getByRole('button', { name: 'Jobs' });
  expect(jobs.closest('header')).toBeNull();
  expect(jobs.closest('.sidebar-footer')).not.toBeNull();
  expect(
    jobs.compareDocumentPosition(screen.getByRole('button', { name: 'Settings' })) &
      Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  const settingsHeading = screen.getByRole('heading', { name: 'Settings' });
  const main = screen.getByRole('main');
  main.scrollTop = 155;
  fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
  expect(main.scrollTop).toBe(0);
  expect(screen.getByRole('heading', { name: 'Jobs', level: 1 })).toHaveFocus();
  expect(screen.queryByRole('dialog', { name: 'Jobs' })).not.toBeInTheDocument();
  expect(jobs).toHaveAttribute('aria-current', 'page');
  expect(jobs).not.toHaveAttribute('aria-haspopup');
  expect(screen.getByText('No running jobs.')).toBeVisible();
  expect(screen.queryByRole('heading', { name: 'Settings' })).not.toBeInTheDocument();
  fireEvent.keyDown(document, { key: 'Escape' });
  expect(screen.getByRole('heading', { name: 'Jobs' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Back from Jobs' }));
  await waitFor(() => expect(screen.getByRole('button', { name: 'Jobs' })).toHaveFocus());
  expect(screen.getByRole('heading', { name: 'Settings' })).toBe(settingsHeading);
  expect(settingsHeading).toBeVisible();
  expect(main.scrollTop).toBe(155);
});

it('keeps the navigation toggle in the sidebar menu across collapse and page changes', async () => {
  render(<App />);
  await ready();
  const collapse = screen.getByRole('button', { name: 'Collapse navigation' });
  const menu = screen.getByRole('navigation', { name: 'Workspace actions' });
  expect(within(menu).getByRole('button', { name: 'Collapse navigation' })).toBe(collapse);
  expect(collapse.closest('header')).toBeNull();
  expect(collapse.previousElementSibling).toHaveAccessibleName('Settings');
  expect(collapse).toHaveAttribute('aria-controls', 'app-sidebar');
  fireEvent.click(collapse);
  expect(screen.getByRole('button', { name: 'Expand navigation' })).toHaveAttribute(
    'aria-expanded',
    'false',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  const expand = screen.getByRole('button', { name: 'Expand navigation' });
  expect(within(menu).getByRole('button', { name: 'Expand navigation' })).toBe(expand);
  expect(expand.closest('header')).toBeNull();
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeVisible();
  fireEvent.click(expand);
  expect(screen.getByRole('button', { name: 'Collapse navigation' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
});

it('keeps a selected-folder draft mounted while Jobs is open and allows normal sidebar navigation', async () => {
  vi.mocked(pickDirectory).mockResolvedValue({
    id: 'draft-folder',
    displayPath: '/generated/draft',
    identityAvailable: true,
  });
  render(<App />);
  await ready();
  fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
  const input = await screen.findByRole('textbox', { name: 'Library name' });
  fireEvent.change(input, { target: { value: 'Unsaved family draft' } });
  fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
  expect(screen.getByRole('heading', { name: 'Jobs' })).toBeVisible();
  expect(input).not.toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Back from Jobs' }));
  expect(screen.getByRole('textbox', { name: 'Library name' })).toBe(input);
  expect(input).toHaveValue('Unsaved family draft');
  expect(library).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeVisible();
  expect(screen.queryByRole('heading', { name: 'Jobs' })).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Jobs' })).not.toHaveAttribute('aria-current');
});

it('suppresses native context menus throughout the sidebar', async () => {
  render(<App />);
  await ready();
  expect(fireEvent.contextMenu(screen.getByRole('button', { name: 'Settings' }))).toBe(false);
  expect(fireEvent.contextMenu(screen.getByLabelText('Application version'))).toBe(false);
});

it('applies the map query only through Show in photos and drops it on ordinary Photos navigation', async () => {
  vi.mocked(libraryData).mockResolvedValue({
    roots: [
      {
        id: 'generated',
        label: 'Generated',
        displayPath: '/generated',
        mode: 'referenced',
        isBase: true,
        availability: 'online',
        filter: fixturePreferences().scanFilter,
        lastScan: null,
        fileCount: 1,
      },
    ],
    jobs: [],
    revision: '1',
    previewGeneration: '0',
    metadataAvailable: true,
    previewAvailable: true,
  });
  render(<App />);
  const photos = await screen.findByRole('button', { name: 'Photos' });
  fireEvent.click(screen.getByRole('button', { name: 'Map' }));
  fireEvent.click(photos);
  expect(screen.getByLabelText('Photo query')).toHaveTextContent('All photos');
  fireEvent.click(screen.getByRole('button', { name: 'Map' }));
  fireEvent.click(screen.getByRole('button', { name: 'Show in photos' }));
  expect(screen.getByLabelText('Photo query')).toHaveTextContent('Map area');
  fireEvent.click(screen.getByRole('button', { name: 'Return to map' }));
  fireEvent.click(photos);
  expect(screen.getByLabelText('Photo query')).toHaveTextContent('All photos');
  fireEvent.click(screen.getByRole('button', { name: 'Map' }));
  fireEvent.click(screen.getByRole('button', { name: 'Show in photos' }));
  expect(screen.getByLabelText('Photo query')).toHaveTextContent('Map area');
  fireEvent.click(photos);
  expect(screen.getByLabelText('Photo query')).toHaveTextContent('All photos');
});
