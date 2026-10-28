import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { BatchToolbar } from './BatchToolbar';
function props() {
  return {
    loading: false,
    total: 2000,
    count: 2,
    busy: false,
    frozen: false,
    canCompare: true,
    onFreeze: vi.fn(),
    onFavorite: vi.fn(),
    onClear: vi.fn(),
    onTags: vi.fn().mockResolvedValue(true),
    onCompare: vi.fn(),
  };
}
it('shows a contextual explicit scope and adds pending tags without replacing existing tags', async () => {
  const callbacks = props();
  render(<BatchToolbar {...callbacks} />);
  expect(screen.getByRole('toolbar', { name: 'Selected photo actions' })).toBeVisible();
  expect(screen.getByText('Explicitly selected photos')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Add tags' }));
  fireEvent.change(screen.getByRole('textbox', { name: 'Tags for selected photos' }), {
    target: { value: 'Family, Trips' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save tags to selection' }));
  await waitFor(() => expect(callbacks.onTags).toHaveBeenCalledWith(['Family, Trips']));
});
it('keeps all-query scope visible and prevents comparing implicit or hidden result members', () => {
  render(<BatchToolbar {...props()} count={2000} frozen canCompare={false} />);
  expect(screen.getByText('All matching results')).toBeVisible();
  expect(screen.getByRole('button', { name: 'Compare' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Export' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Add to album' })).toBeDisabled();
});
it('offers whole-query selection only as an explicit action when nothing is selected', () => {
  const callbacks = props();
  render(<BatchToolbar {...callbacks} count={0} />);
  expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
  expect(callbacks.onFreeze).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Select all results' }));
  expect(callbacks.onFreeze).toHaveBeenCalledOnce();
  expect(callbacks.onFavorite).not.toHaveBeenCalled();
});
