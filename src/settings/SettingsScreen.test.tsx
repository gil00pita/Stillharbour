import { fireEvent, render, screen, waitFor, within } from '../test/render';
import { expect, it, vi } from 'vitest';
import type { ComponentProps } from 'react';
import { SettingsScreen } from './SettingsScreen';
import { fixturePreferences } from '../library/__tests__/fixtures';
import type { LibraryStatus } from '../bindings';
const status: LibraryStatus = {
  roots: [
    {
      id: 'root',
      label: 'Generated photos',
      displayPath: 'Generated root',
      mode: 'referenced',
      isBase: true,
      availability: 'offline',
      filter: fixturePreferences().scanFilter,
      lastScan: null,
      fileCount: 20,
    },
  ],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
function setup(
  onSave = vi.fn(async (value) => value),
  overrides: Partial<ComponentProps<typeof SettingsScreen>> = {},
) {
  const onRelink = vi.fn(async () => {});
  const onAddFolder = vi.fn();
  const onRescan = vi.fn(async () => {});
  render(
    <SettingsScreen
      preferences={fixturePreferences()}
      status={status}
      onSave={onSave}
      onRelink={onRelink}
      onAddFolder={onAddFolder}
      initialRootId={null}
      onRescan={onRescan}
      onMaintenance={vi.fn(async () => {})}
      locationBusy={false}
      onError={vi.fn()}
      {...overrides}
    />,
  );
  return { onSave, onRelink, onAddFolder, onRescan };
}
it('saves and clears a catalog-scoped import hint without changing the browsing default or grants', async () => {
  const managed = {
    ...status.roots[0]!,
    id: 'destination',
    label: 'Managed copies',
    mode: 'managed' as const,
    availability: 'online',
  };
  const saveImport = vi.fn(async (rootId: string | null) => ({
    catalogId: 'current-catalog',
    rootId,
  }));
  const c = setup(undefined, {
    catalogId: 'current-catalog',
    status: { ...status, roots: [...status.roots, managed] },
    onSaveImportDestination: saveImport,
  });
  expect(screen.getByRole('option', { name: 'Generated photos — read-only' })).toBeDisabled();
  fireEvent.change(screen.getByRole('combobox', { name: 'Preferred import destination' }), {
    target: { value: 'destination' },
  });
  expect(c.onSave).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Save import destination' }));
  await screen.findByText(
    'Import destination saved for this catalog. Folder grants and existing plans are unchanged.',
  );
  expect(screen.getByRole('combobox', { name: 'Preferred import destination' })).toHaveValue(
    'destination',
  );
  expect(saveImport).toHaveBeenLastCalledWith('destination');
  expect(c.onSave).not.toHaveBeenCalled();
  fireEvent.change(screen.getByRole('combobox', { name: 'Preferred import destination' }), {
    target: { value: '' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save import destination' }));
  await screen.findByText(
    'Import destination saved for this catalog. Folder grants and existing plans are unchanged.',
  );
  expect(c.onSave).not.toHaveBeenCalled();
  expect(saveImport).toHaveBeenLastCalledWith(null);
  expect(c.onRescan).not.toHaveBeenCalled();
  expect(c.onRelink).not.toHaveBeenCalled();
});
it.each(['disk failure', 'foreign catalog'])(
  'retains an import choice after %s without claiming success',
  async (failure) => {
    const managed = {
      ...status.roots[0]!,
      id: 'destination',
      label: 'Managed copies',
      mode: 'managed' as const,
    };
    const saveImport = vi.fn(async () => {
      if (failure === 'disk failure') throw new Error('Catalog storage unavailable');
      return { catalogId: 'another-catalog', rootId: 'destination' };
    });
    const c = setup(undefined, {
      catalogId: 'current-catalog',
      status: { ...status, roots: [...status.roots, managed] },
      onSaveImportDestination: saveImport,
    });
    fireEvent.change(screen.getByRole('combobox', { name: 'Preferred import destination' }), {
      target: { value: 'destination' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Save import destination' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      failure === 'disk failure' ? 'Catalog storage unavailable' : 'another catalog or choice',
    );
    expect(screen.getByRole('combobox', { name: 'Preferred import destination' })).toHaveValue(
      'destination',
    );
    expect(screen.getByRole('button', { name: 'Save import destination' })).toBeEnabled();
    expect(screen.queryByText(/Import destination saved for this catalog/)).not.toBeInTheDocument();
    expect(c.onSave).not.toHaveBeenCalled();
  },
);
it('shows scan settings inline and uses existing library IDs without saving defaults', () => {
  const c = setup();
  expect(screen.getByRole('heading', { name: 'Settings' })).toHaveFocus();
  fireEvent.click(screen.getByRole('button', { name: 'Change location for Generated photos' }));
  expect(c.onRelink).toHaveBeenCalledWith('root');
  expect(screen.getByRole('heading', { name: 'Scan settings' })).toBeVisible();
  expect(
    screen.queryByRole('button', { name: 'Scan settings for Generated photos' }),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('combobox', { name: 'Library to configure' })).toHaveValue('root');
  expect(c.onRescan).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Add photo folder' }));
  expect(c.onAddFolder).toHaveBeenCalledOnce();
  expect(c.onSave).not.toHaveBeenCalled();
});
it('keeps view choices as a draft and saves pending tags with new-folder defaults', async () => {
  const c = setup();
  fireEvent.click(screen.getByRole('button', { name: 'Uniform grid' }));
  expect(c.onSave).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('switch', { name: 'Read only by default' }));
  fireEvent.change(screen.getByRole('combobox', { name: 'Default library' }), {
    target: { value: 'root' },
  });
  fireEvent.click(
    within(screen.getByRole('form', { name: 'Application defaults' })).getByText('Advanced'),
  );
  fireEvent.change(defaults().getByRole('textbox', { name: 'File Extensions' }), {
    target: { value: '.PNG' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save defaults' }));
  await waitFor(() =>
    expect(c.onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        galleryView: 'grid',
        rootMode: 'referenced',
        defaultRootId: 'root',
        scanFilter: expect.objectContaining({ extensions: ['jpg', 'png'] }),
      }),
    ),
  );
  expect(
    await screen.findByText('Defaults saved. Existing photo folders are unchanged.'),
  ).toBeVisible();
});
it('retains unsaved values and reports a failed native save without success', async () => {
  const c = setup(
    vi.fn(async () => {
      throw new Error('Settings disk is unavailable');
    }),
  );
  fireEvent.click(screen.getByRole('switch', { name: 'Read only by default' }));
  fireEvent.click(screen.getByRole('button', { name: 'Save defaults' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Settings disk is unavailable');
  expect(screen.getByRole('switch', { name: 'Read only by default' })).toBeChecked();
  expect(
    screen.queryByText('Defaults saved. Existing photo folders are unchanged.'),
  ).not.toBeInTheDocument();
  expect(c.onSave).toHaveBeenCalledOnce();
});

it.each(['5', '-1'])('discards size amount/unit/validity and pending tags (%s)', async (amount) => {
  const c = setup();
  fireEvent.click(
    within(screen.getByRole('form', { name: 'Application defaults' })).getByText('Advanced'),
  );
  fireEvent.change(defaults().getByRole('textbox', { name: 'Minimum size' }), {
    target: { value: amount },
  });
  fireEvent.change(defaults().getByRole('combobox', { name: 'Minimum size unit' }), {
    target: { value: 'GB' },
  });
  fireEvent.change(defaults().getByRole('textbox', { name: 'File Extensions' }), {
    target: { value: '.webp' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Discard changes' }));
  const advanced = within(screen.getByRole('form', { name: 'Application defaults' })).getByText(
    'Advanced',
  );
  if (!advanced.closest('details')?.open) fireEvent.click(advanced);
  expect(defaults().getByRole('textbox', { name: 'Minimum size' })).toHaveValue('');
  expect(defaults().getByRole('textbox', { name: 'Minimum size' })).toHaveAttribute(
    'aria-invalid',
    'false',
  );
  expect(defaults().getByRole('combobox', { name: 'Minimum size unit' })).toHaveValue('MB');
  expect(defaults().getByRole('textbox', { name: 'File Extensions' })).toHaveValue('');
  fireEvent.click(screen.getByRole('button', { name: 'Save defaults' }));
  await waitFor(() =>
    expect(c.onSave).toHaveBeenCalledWith(
      expect.objectContaining({
        scanFilter: expect.objectContaining({ minBytes: null, extensions: ['jpg'] }),
      }),
    ),
  );
});

function defaults() {
  return within(screen.getByRole('form', { name: 'Application defaults' }));
}

it.each([
  [null, 'Open a catalog to choose its preferred import destination'],
  ['current-catalog', 'Saving an import destination is unavailable in this session'],
])('explains unavailable import preferences for catalog %s', (catalogId, reason) => {
  setup(undefined, { catalogId });
  const destination = screen.getByRole('combobox', { name: 'Preferred import destination' });
  expect(destination).toBeDisabled();
  expect(destination).toHaveAccessibleDescription(reason);
  expect(screen.getByText(reason)).toBeVisible();
  expect(screen.getByRole('button', { name: 'Save import destination' })).toBeDisabled();
});

it('returns focus after cancel even when animation frames are suspended', async () => {
  vi.stubGlobal('requestAnimationFrame', vi.fn());
  setup();
  const opener = screen.getByRole('button', { name: 'Remove library Generated photos' });
  fireEvent.click(opener);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Cancel' })).toBeEnabled());
  fireEvent.keyDown(screen.getByRole('form', { name: 'Review library maintenance' }), {
    key: 'Escape',
  });
  await waitFor(() => expect(opener).toHaveFocus());
  vi.unstubAllGlobals();
});
