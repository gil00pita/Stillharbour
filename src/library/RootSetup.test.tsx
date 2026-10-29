import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LibraryRoot } from '../bindings';
import { libraryData } from '../ipc';
import { defaultFilter } from './model';
import { RootSetup } from './RootSetup';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(libraryData).mockReset().mockResolvedValue([]);
});
const selected = {
  id: 'generated',
  displayPath: 'Generated test library',
  identityAvailable: true,
};
const expectedExtensions =
  'jpg jpeg jfif heic heif hif png tif tiff webp avif gif bmp dng cr3 cr2 crw nef nrw arw sr2 srf raf orf rw2 raw pef srw x3f 3fr fff iiq rwl'.split(
    ' ',
  );
const existingRoot: LibraryRoot = {
  id: 'existing',
  label: 'Existing folder',
  displayPath: 'Generated root',
  mode: 'referenced',
  isBase: true,
  availability: 'online',
  filter: { ...defaultFilter, extensions: [] },
  lastScan: null,
  fileCount: 1,
};
function setup(root?: LibraryRoot) {
  const onSave = vi.fn();
  render(
    <RootSetup
      selected={selected}
      root={root}
      busy={false}
      onSave={onSave}
      onCancel={vi.fn()}
      onError={vi.fn()}
    />,
  );
  return onSave;
}
function submit() {
  fireEvent.submit(screen.getByRole('button', { name: /Save &/ }).closest('form')!);
}
function expand() {
  fireEvent.click(screen.getByText('Advanced'));
}
function add(label: string, value: string) {
  const input = screen.getByRole('textbox', { name: label });
  fireEvent.change(input, { target: { value } });
  fireEvent.keyDown(input, { key: 'Enter' });
  return input;
}

describe('advanced folder setup', () => {
  it('defaults to Managed and the exact 33-photo preset with independent hidden switches', () => {
    const onSave = setup();
    expand();
    expect(screen.getByRole('switch', { name: 'Read only' })).not.toBeChecked();
    expect(screen.queryByRole('combobox', { name: /Folder mode/ })).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Include patterns')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('From date')).not.toBeInTheDocument();
    expect(screen.queryByLabelText('Through date')).not.toBeInTheDocument();
    for (const ext of expectedExtensions)
      expect(
        screen.getByRole('button', { name: `Remove .${ext} from File Extensions` }),
      ).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: 'Include hidden folders' }));
    submit();
    expect(onSave).toHaveBeenCalledWith(
      'Photo library',
      'managed',
      expect.objectContaining({
        includeHiddenFolders: true,
        includeHiddenFiles: false,
        includeGlobs: [],
        extensions: expectedExtensions,
        fromDate: null,
        throughDate: null,
      }),
    );
    fireEvent.click(screen.getByRole('switch', { name: 'Read only' }));
    submit();
    expect(onSave).toHaveBeenLastCalledWith('Photo library', 'referenced', expect.any(Object));
  });
  it('preserves saved mode/custom extensions and clears removed include/date restrictions', () => {
    const onSave = setup({
      ...existingRoot,
      filter: {
        ...existingRoot.filter,
        includeGlobs: ['Trips/**'],
        extensions: ['JPG', 'png'],
        fromDate: '2024-01-01',
        throughDate: '2024-12-31',
      },
    });
    expand();
    expect(screen.getByRole('switch', { name: 'Read only' })).toBeChecked();
    expect(screen.getByRole('button', { name: 'Remove .jpg from File Extensions' })).toBeVisible();
    submit();
    expect(onSave).toHaveBeenCalledWith(
      'Existing folder',
      'referenced',
      expect.objectContaining({
        includeGlobs: [],
        extensions: ['jpg', 'png'],
        fromDate: null,
        throughDate: null,
      }),
    );
  });
  it('edits/removes tags, deduplicates extension case/dots, and keeps commas in exclusion rules', () => {
    const onSave = setup(existingRoot);
    expand();
    add('Exclude patterns', '**/{edited,exported}/**');
    add('Excluded folder names', 'Private, Exports');
    add('File Extensions', '.JPG');
    add('File Extensions', 'jpg');
    expect(
      screen.getAllByRole('button', { name: 'Remove .jpg from File Extensions' }),
    ).toHaveLength(1);
    const input = screen.getByRole('textbox', { name: 'File Extensions' });
    fireEvent.keyDown(input, { key: 'Backspace' });
    expect(input).toHaveValue('.jpg');
    fireEvent.change(input, { target: { value: '.PNG' } });
    fireEvent.keyDown(input, { key: 'Tab' });
    fireEvent.click(screen.getByRole('button', { name: 'Remove .png from File Extensions' }));
    submit();
    expect(onSave).toHaveBeenLastCalledWith(
      'Existing folder',
      'referenced',
      expect.objectContaining({
        excludeGlobs: ['**/{edited,exported}/**'],
        excludedFolders: ['Private, Exports'],
        extensions: [],
      }),
    );
  });
  it('includes and normalizes pending values in Save/Preview without blur', () => {
    const onSave = setup(existingRoot);
    expand();
    fireEvent.change(screen.getByRole('textbox', { name: 'Excluded folder names' }), {
      target: { value: 'Private' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'File Extensions' }), {
      target: { value: '.JPG' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Preview filters' }));
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({
        kind: 'filter_preview',
        filter: expect.objectContaining({
          excludedFolders: ['Private'],
          extensions: ['jpg'],
          includeGlobs: [],
          fromDate: null,
          throughDate: null,
        }),
      }),
      'filter_preview',
    );
    submit();
    expect(onSave).toHaveBeenCalledWith(
      'Existing folder',
      'referenced',
      expect.objectContaining({ extensions: ['jpg'] }),
    );
  });
  it('supports multiline extension paste and composition without submitting the form', () => {
    const onSave = setup(existingRoot);
    expand();
    const input = screen.getByRole('textbox', { name: 'File Extensions' });
    fireEvent.paste(input, { clipboardData: { getData: () => '.JPG\n.png\nwebp' } });
    fireEvent.change(input, { target: { value: '.heic' } });
    fireEvent.keyDown(input, { key: 'Enter', isComposing: true });
    expect(input).toHaveValue('.heic');
    expect(onSave).not.toHaveBeenCalled();
    fireEvent.blur(input);
    submit();
    expect(onSave).toHaveBeenCalledWith(
      'Existing folder',
      'referenced',
      expect.objectContaining({ extensions: ['jpg', 'png', 'webp', 'heic'] }),
    );
  });
  it('starts collapsed and retains values when Advanced is closed and reopened', () => {
    setup(existingRoot);
    const summary = screen.getByText('Advanced');
    const accordion = summary.closest('details');
    expect(accordion).not.toHaveAttribute('open');
    expand();
    add('File Extensions', '.png');
    expand();
    expect(accordion).not.toHaveAttribute('open');
    expand();
    expect(screen.getByRole('button', { name: 'Remove .png from File Extensions' })).toBeVisible();
  });
  it('sends exact size thresholds to both Preview and Save', () => {
    const onSave = setup();
    expand();
    fireEvent.change(screen.getByRole('textbox', { name: 'Minimum size' }), {
      target: { value: '1.5' },
    });
    fireEvent.change(screen.getByRole('textbox', { name: 'Maximum size' }), {
      target: { value: '2' },
    });
    fireEvent.change(screen.getByRole('combobox', { name: 'Maximum size unit' }), {
      target: { value: 'GB' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Preview filters' }));
    expect(libraryData).toHaveBeenCalledWith(
      expect.objectContaining({
        filter: expect.objectContaining({ minBytes: '1500000', maxBytes: '2000000000' }),
      }),
      'filter_preview',
    );
    submit();
    expect(onSave).toHaveBeenLastCalledWith(
      'Photo library',
      'managed',
      expect.objectContaining({ minBytes: '1500000', maxBytes: '2000000000' }),
    );
  });
  it('blocks Save and Preview while a size draft is invalid', () => {
    const onSave = setup();
    expand();
    fireEvent.change(screen.getByRole('textbox', { name: 'Minimum size' }), {
      target: { value: '-1' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Preview filters' }));
    submit();
    expect(libraryData).not.toHaveBeenCalled();
    expect(onSave).not.toHaveBeenCalled();
    fireEvent.change(screen.getByRole('textbox', { name: 'Minimum size' }), {
      target: { value: '' },
    });
    submit();
    expect(onSave).toHaveBeenCalledWith(
      'Photo library',
      'managed',
      expect.objectContaining({ minBytes: null }),
    );
  });
  it('retains existing byte-exact limits on an untouched settings save', () => {
    const onSave = setup({
      ...existingRoot,
      filter: { ...existingRoot.filter, minBytes: '1501', maxBytes: '9007199254740993' },
    });
    expand();
    expect(screen.getByRole('textbox', { name: 'Minimum size' })).toHaveValue('1.501');
    expect(screen.getByRole('combobox', { name: 'Minimum size unit' })).toHaveValue('KB');
    submit();
    expect(onSave).toHaveBeenLastCalledWith(
      'Existing folder',
      'referenced',
      expect.objectContaining({ minBytes: '1501', maxBytes: '9007199254740993' }),
    );
  });
});

it('uses saved defaults for new folders and keeps existing root settings', () => {
  const defaults = {
    rootMode: 'managed' as const,
    scanFilter: { ...defaultFilter, includeHiddenFiles: true, extensions: ['png'] },
  };
  const c = { busy: false, onSave: vi.fn(), onCancel: vi.fn(), onError: vi.fn(), defaults };
  const { rerender } = render(<RootSetup selected={selected} {...c} />);
  fireEvent.click(screen.getByText('Advanced'));
  expect(screen.getByRole('switch', { name: 'Include hidden files' })).toBeChecked();
  expect(screen.getByRole('button', { name: 'Remove .png from File Extensions' })).toBeVisible();
  rerender(<RootSetup key="existing" root={existingRoot} {...c} />);
  expect(screen.getByRole('switch', { name: 'Read only' })).toBeChecked();
  fireEvent.click(screen.getByText('Advanced'));
  expect(screen.getByRole('switch', { name: 'Include hidden files' })).not.toBeChecked();
  expect(
    screen.queryByRole('button', { name: 'Remove .png from File Extensions' }),
  ).not.toBeInTheDocument();
});
