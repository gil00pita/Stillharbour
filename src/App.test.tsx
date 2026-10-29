import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App';
import { fixturePreferences } from './library/__tests__/fixtures';
import { getStatus, library, libraryData, pickDirectory, getPreferences } from './ipc';

vi.mock('./ipc', async (original) => ({
  ...(await original<typeof import('./ipc')>()),
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
    expect(await screen.findByRole('alert')).toHaveTextContent('Desktop connection unavailable.');
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
    fireEvent.click(screen.getByRole('button', { name: 'Choose a folder' }));
    await screen.findByText('Selected folder');
    expect(screen.getByRole('textbox', { name: 'Library name' })).toHaveFocus();
    expect(screen.getByText('Not scanned')).toBeVisible();
    expect(library).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'Choose a folder' })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Start by selecting your Photo library folder.' }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText('Local by design.')).not.toBeInTheDocument();
    expect(screen.getByLabelText('Application version')).toHaveTextContent('Version 0.1.0');
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

it('opens Jobs in the current header without a sidebar destination or changing Settings', async () => {
  render(<App />);
  await ready();
  const jobs = screen.getByRole('button', { name: 'Jobs' });
  expect(jobs.closest('header')).not.toBeNull();
  expect(jobs.closest('.sidebar')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  fireEvent.click(screen.getByRole('button', { name: 'Jobs' }));
  expect(screen.getByRole('dialog', { name: 'Jobs' })).toBeVisible();
  expect(screen.getByText('No running jobs.')).toBeVisible();
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeVisible();
  fireEvent.keyDown(document, { key: 'Escape' });
  expect(screen.getByRole('button', { name: 'Jobs' })).toHaveFocus();
  expect(screen.getByRole('heading', { name: 'Settings' })).toBeVisible();
});

it('keeps an icon-only collapse control before the current header title and out of the sidebar', async () => {
  render(<App />);
  await ready();
  const collapse = screen.getByRole('button', { name: 'Collapse navigation' });
  expect(collapse.closest('header')).not.toBeNull();
  expect(collapse.nextElementSibling).toHaveTextContent('Your library');
  expect(collapse.textContent).toBe('');
  expect(collapse).toHaveAttribute('aria-controls', 'app-sidebar');
  expect(
    within(screen.getByRole('complementary')).queryByRole('button', { name: /navigation/ }),
  ).not.toBeInTheDocument();
  fireEvent.click(collapse);
  expect(screen.getByRole('button', { name: 'Expand navigation' })).toHaveAttribute(
    'aria-expanded',
    'false',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  const expand = screen.getByRole('button', { name: 'Expand navigation' });
  expect(expand.closest('header')).not.toBeNull();
  expect(expand.nextElementSibling).toHaveTextContent('Settings');
  fireEvent.click(expand);
  expect(screen.getByRole('button', { name: 'Collapse navigation' })).toHaveAttribute(
    'aria-expanded',
    'true',
  );
});
