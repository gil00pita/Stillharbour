import { fireEvent, render, screen } from '../../test/render';
import { beforeEach, expect, it, vi } from 'vitest';
import { InspectionDialog } from './InspectionDialog';

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});

it('closes on Escape from a focused child without relying on a native cancel event', () => {
  const close = vi.fn();
  render(
    <InspectionDialog title="Measurement" onClose={close}>
      <button>Scan settings</button>
    </InspectionDialog>,
  );
  const control = screen.getByRole('button', { name: 'Scan settings' });
  control.focus();
  expect(control).toHaveFocus();
  fireEvent.keyDown(control, { key: 'Escape' });
  expect(close).toHaveBeenCalledOnce();
});

it('leaves Escape consumed by a child control to that control', () => {
  const close = vi.fn();
  render(
    <InspectionDialog title="Measurement" onClose={close}>
      <button onKeyDown={(event) => event.preventDefault()}>Nested control</button>
    </InspectionDialog>,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Nested control' }), { key: 'Escape' });
  expect(close).not.toHaveBeenCalled();
  expect(screen.getByRole('dialog')).toBeInTheDocument();
});

it('keeps a busy dialog open for both Escape and native cancel, then permits dismissal', () => {
  const close = vi.fn();
  const { rerender } = render(
    <InspectionDialog title="Saving album" busy onClose={close}>
      <button>Photo picker</button>
    </InspectionDialog>,
  );
  const dialog = screen.getByRole('dialog');
  fireEvent.keyDown(screen.getByRole('button', { name: 'Photo picker' }), { key: 'Escape' });
  fireEvent(dialog, new Event('cancel', { cancelable: true }));
  expect(close).not.toHaveBeenCalled();
  expect(dialog).toHaveAttribute('open');
  expect(screen.getByRole('button', { name: 'Close' })).toBeDisabled();

  rerender(
    <InspectionDialog title="Saving album" onClose={close}>
      <button>Photo picker</button>
    </InspectionDialog>,
  );
  fireEvent.keyDown(screen.getByRole('button', { name: 'Photo picker' }), { key: 'Escape' });
  expect(close).toHaveBeenCalledOnce();
});
