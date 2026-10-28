import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ScanSettingsGroup } from './ScanSettingsGroup';
import { fixturePreferences } from '../library/__tests__/fixtures';
import type { LibraryRoot } from '../bindings';
const roots: LibraryRoot[] = ['One', 'Two'].map((label) => ({
  id: label.toLowerCase(),
  label,
  displayPath: `Generated ${label}`,
  mode: 'managed',
  isBase: label === 'One',
  availability: 'online',
  filter: fixturePreferences().scanFilter,
  lastScan: null,
  fileCount: 2,
}));
function setup(onRescan = vi.fn(async () => {})) {
  render(
    <ScanSettingsGroup
      roots={roots}
      initialRootId="two"
      busy={false}
      onRescan={onRescan}
      onError={vi.fn()}
    />,
  );
  return { onRescan, group: within(screen.getByRole('region', { name: 'Scan settings' })) };
}
it('starts with the requested library and submits its ID with pending scan edits only on explicit rescan', async () => {
  const { group, onRescan } = setup();
  expect(group.getByRole('heading', { name: 'Scan settings' })).toHaveFocus();
  expect(group.getByRole('combobox', { name: 'Library to configure' })).toHaveValue('two');
  fireEvent.click(group.getByText('Advanced'));
  fireEvent.change(group.getByRole('textbox', { name: 'File Extensions' }), {
    target: { value: '.WEBP' },
  });
  fireEvent.click(group.getByRole('switch', { name: 'Read only' }));
  expect(onRescan).not.toHaveBeenCalled();
  fireEvent.click(group.getByRole('button', { name: 'Save & Rescan' }));
  await waitFor(() =>
    expect(onRescan).toHaveBeenCalledWith(
      'two',
      'referenced',
      expect.objectContaining({ extensions: ['jpg', 'webp'] }),
    ),
  );
  expect(await group.findByRole('status')).toHaveTextContent('Scan started');
});
it('retains failed drafts and discards amount/unit/validity, pending tags and mode without starting a scan', async () => {
  const { group, onRescan } = setup(
    vi.fn(async () => {
      throw new Error('Generated folder offline');
    }),
  );
  fireEvent.click(group.getByRole('switch', { name: 'Read only' }));
  fireEvent.click(group.getByRole('button', { name: 'Save & Rescan' }));
  expect(await group.findByRole('alert')).toHaveTextContent('Generated folder offline');
  expect(group.getByRole('switch', { name: 'Read only' })).toBeChecked();
  expect(group.queryByRole('status')).not.toBeInTheDocument();
  fireEvent.click(group.getByText('Advanced'));
  fireEvent.change(group.getByRole('textbox', { name: 'Minimum size' }), {
    target: { value: '-1' },
  });
  fireEvent.change(group.getByRole('combobox', { name: 'Minimum size unit' }), {
    target: { value: 'GB' },
  });
  fireEvent.change(group.getByRole('textbox', { name: 'File Extensions' }), {
    target: { value: '.png' },
  });
  fireEvent.click(group.getByRole('button', { name: 'Discard scan changes' }));
  expect(group.getByRole('switch', { name: 'Read only' })).not.toBeChecked();
  fireEvent.click(group.getByText('Advanced'));
  expect(group.getByRole('textbox', { name: 'Minimum size' })).toHaveValue('');
  expect(group.getByRole('textbox', { name: 'Minimum size' })).toHaveAttribute(
    'aria-invalid',
    'false',
  );
  expect(group.getByRole('combobox', { name: 'Minimum size unit' })).toHaveValue('MB');
  expect(group.getByRole('textbox', { name: 'File Extensions' })).toHaveValue('');
  expect(group.queryByRole('alert')).not.toBeInTheDocument();
  expect(onRescan).toHaveBeenCalledOnce();
});
it('switches libraries without carrying unsaved scan edits into another root', () => {
  const { group, onRescan } = setup();
  fireEvent.click(group.getByRole('switch', { name: 'Read only' }));
  fireEvent.change(group.getByRole('combobox', { name: 'Library to configure' }), {
    target: { value: 'one' },
  });
  expect(group.getByRole('textbox', { name: 'Library name' })).toHaveValue('One');
  expect(group.getByRole('switch', { name: 'Read only' })).not.toBeChecked();
  expect(onRescan).not.toHaveBeenCalled();
});
it('disables starting another scan while the selected library is scanning', () => {
  const onRescan = vi.fn(async () => {});
  render(
    <ScanSettingsGroup
      roots={[{ ...roots[0]!, availability: 'scanning' }]}
      initialRootId={null}
      busy={false}
      onRescan={onRescan}
      onError={vi.fn()}
    />,
  );
  expect(screen.getByRole('button', { name: 'Scan running…' })).toBeDisabled();
  expect(onRescan).not.toHaveBeenCalled();
});
