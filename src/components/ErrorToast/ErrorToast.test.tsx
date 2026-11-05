import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { afterEach, expect, it, vi } from 'vitest';
import { ErrorToast } from './ErrorToast';

afterEach(() => vi.useRealTimers());

it('portals the complete error into a bottom toast and preserves it until dismissal', async () => {
  const dismiss = vi.fn();
  const message =
    'The catalog request queue is full (16 waiting requests).\nThis request was not queued. Wait a moment, then try again.';
  const { container } = render(<ErrorToast error={message} onDismiss={dismiss} />);
  const toast = await screen.findByRole('alert');
  expect(toast).toHaveTextContent('16 waiting requests');
  expect(toast).toHaveTextContent('This request was not queued.');
  expect(toast).toHaveAttribute('data-placement', 'bottom');
  expect(container).not.toContainElement(toast);
  vi.useFakeTimers();
  act(() => vi.advanceTimersByTime(120_000));
  expect(toast).toBeInTheDocument();
  vi.useRealTimers();
  fireEvent.click(screen.getByRole('button', { name: 'Dismiss error' }));
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  expect(dismiss).toHaveBeenCalledTimes(1);
});

it('keeps repeated errors in one toast and replaces details without accumulating a queue', async () => {
  const dismiss = vi.fn();
  const view = render(<ErrorToast error="Catalog is busy" onDismiss={dismiss} />);
  const original = await screen.findByRole('alert');
  for (let i = 0; i < 10; i++)
    view.rerender(<ErrorToast error="Catalog is busy" onDismiss={dismiss} />);
  expect(screen.getAllByRole('alert')).toHaveLength(1);
  expect(screen.getByRole('alert')).toBe(original);
  view.rerender(<ErrorToast error="Catalog worker stopped" onDismiss={dismiss} />);
  await screen.findByText('Catalog worker stopped');
  expect(screen.queryByText('Catalog is busy')).not.toBeInTheDocument();
  expect(screen.getAllByRole('alert')).toHaveLength(1);
  view.rerender(<ErrorToast error={null} onDismiss={dismiss} />);
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
});

it('retains the explicit reconnect action and removes notifications when the session unmounts', async () => {
  const reconnect = vi.fn();
  const view = render(
    <ErrorToast error="Connection unavailable" onDismiss={vi.fn()} onReconnect={reconnect} />,
  );
  await screen.findByRole('alert');
  fireEvent.click(screen.getByRole('button', { name: 'Reconnect' }));
  expect(reconnect).toHaveBeenCalledTimes(1);
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  view.rerender(<ErrorToast error="Another error" onDismiss={vi.fn()} />);
  await screen.findByRole('alert');
  view.unmount();
  expect(screen.queryByRole('alert')).not.toBeInTheDocument();
});

it('does not steal focus and allows Escape dismissal from the notification', async () => {
  const dismiss = vi.fn();
  const view = render(
    <>
      <input aria-label="Search fixture" />
      <ErrorToast error={null} onDismiss={dismiss} />
    </>,
  );
  const input = screen.getByRole('textbox');
  input.focus();
  view.rerender(
    <>
      <input aria-label="Search fixture" />
      <ErrorToast error="Catalog unavailable" onDismiss={dismiss} />
    </>,
  );
  const toast = await screen.findByRole('alert');
  expect(input).toHaveFocus();
  toast.focus();
  const outsideKey = vi.fn();
  window.addEventListener('keydown', outsideKey);
  try {
    fireEvent.keyDown(toast, { key: 'Escape' });
    expect(outsideKey).not.toHaveBeenCalled();
  } finally {
    window.removeEventListener('keydown', outsideKey);
  }
  await waitFor(() => expect(screen.queryByRole('alert')).not.toBeInTheDocument());
  expect(dismiss).toHaveBeenCalledTimes(1);
});
