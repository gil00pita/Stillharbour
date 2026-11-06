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
