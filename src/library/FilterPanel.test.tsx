import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { FilterPanel } from './FilterPanel';
import { useLibraryController } from './useLibraryController';
import { libraryData } from '../ipc';
vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const status = {
  roots: [],
  jobs: [],
  revision: '1',
  previewGeneration: '0',
  metadataAvailable: true,
  previewAvailable: true,
};
const onError = vi.fn();
const onRefresh = vi.fn(async () => {});
beforeEach(() => {
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = true;
    },
  });
  Object.defineProperty(HTMLDialogElement.prototype, 'close', {
    configurable: true,
    value: function (this: HTMLDialogElement) {
      this.open = false;
    },
  });
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind === 'query')
        return { assets: [], total: 5, nextCursor: null, revision: '1' };
      if (request.kind === 'date_buckets') return [];
      if (request.kind === 'freeze_selection') return { id: 'token', count: 5, revision: '1' };
      throw new Error('Unexpected');
    });
});
function setup() {
  const onApply = vi.fn();
  const onClose = vi.fn();
  function Harness() {
    const c = useLibraryController({ status, section: 'Photos', onError, onRefresh });
    return (
      <FilterPanel
        controller={c}
        status={status}
        section="Photos"
        mode="gallery"
        display={{ size: 'comfortable', fit: 'contain' }}
        onApply={onApply}
        onClose={onClose}
        onSettings={vi.fn()}
        onRelink={vi.fn(async () => {})}
      />
    );
  }
  render(<Harness />);
  return { onApply, onClose };
}
it('keeps filter/display choices as a draft until Apply and preserves them on validation', async () => {
  const c = setup();
  await screen.findByText(/5 matching photos/);
  fireEvent.change(screen.getByRole('textbox', { name: 'Tag' }), { target: { value: 'Family' } });
  fireEvent.click(screen.getByRole('button', { name: 'Uniform grid' }));
  expect(c.onApply).not.toHaveBeenCalled();
  expect(screen.getByRole('button', { name: 'Select all results' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Apply filters' }));
  expect(c.onApply).toHaveBeenCalledWith(expect.objectContaining({ tag: 'Family' }), 'grid', {
    size: 'comfortable',
    fit: 'contain',
  });
  expect(c.onClose).toHaveBeenCalledOnce();
});
it('closes on Escape without applying edits', async () => {
  const c = setup();
  await screen.findByText(/5 matching photos/);
  fireEvent.change(screen.getByRole('textbox', { name: 'Tag' }), { target: { value: 'Draft' } });
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { bubbles: true, cancelable: true }));
  expect(c.onClose).toHaveBeenCalledOnce();
  expect(c.onApply).not.toHaveBeenCalled();
});
it('resets filter draft without prematurely changing the live query', async () => {
  const c = setup();
  await screen.findByText(/5 matching photos/);
  fireEvent.change(screen.getByRole('textbox', { name: 'Tag' }), { target: { value: 'Draft' } });
  fireEvent.click(screen.getByRole('button', { name: 'Reset filters' }));
  expect(screen.getByRole('textbox', { name: 'Tag' })).toHaveValue('');
  expect(c.onApply).not.toHaveBeenCalled();
});
it('exposes explicit all-query selection and waits for native confirmation', async () => {
  const c = setup();
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Select all results' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Select all results' }));
  await waitFor(() => expect(c.onClose).toHaveBeenCalledOnce());
  expect(libraryData).toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'freeze_selection' }),
    'selection',
  );
});
