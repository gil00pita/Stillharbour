import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { DisplayPanel } from '@/components/DisplayPanel';

beforeEach(() => {
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});

function setup(busy = false) {
  const onApply = vi.fn();
  const onClose = vi.fn();
  const view = render(
    <DisplayPanel
      mode="gallery"
      display={{ size: 'comfortable', fit: 'contain' }}
      busy={busy}
      onApply={onApply}
      onClose={onClose}
    />,
  );
  return { ...view, onApply, onClose };
}

it('drafts view, zoom and grid fit together until Apply', () => {
  const { onApply, onClose } = setup();
  expect(screen.getByRole('dialog', { name: 'Display options' })).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Uniform grid' }));
  fireEvent.change(screen.getByRole('slider', { name: 'Thumbnail scale' }), {
    target: { value: '2' },
  });
  fireEvent.change(screen.getByRole('combobox', { name: 'Grid image fit' }), {
    target: { value: 'cover' },
  });
  expect(onApply).not.toHaveBeenCalled();
  expect(screen.getByText('Large')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Apply display' }));
  expect(onApply).toHaveBeenCalledWith('grid', { size: 'large', fit: 'cover' });
  expect(onClose).toHaveBeenCalledOnce();
});

it('hides image-only controls in Text list and retains their draft when switching back', () => {
  setup();
  fireEvent.change(screen.getByRole('slider'), { target: { value: '0' } });
  fireEvent.click(screen.getByRole('button', { name: 'Text list' }));
  expect(screen.queryByRole('slider')).not.toBeInTheDocument();
  expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Uniform grid' }));
  expect(screen.getByRole('slider')).toHaveValue('0');
  expect(screen.getByRole('combobox', { name: 'Grid image fit' })).toHaveValue('contain');
});

it('cancels changes with Escape and restores the opener on unmount', () => {
  const opener = document.createElement('button');
  document.body.append(opener);
  opener.focus();
  const { onApply, onClose, unmount } = setup();
  fireEvent.change(screen.getByRole('slider'), { target: { value: '2' } });
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(onClose).toHaveBeenCalledOnce();
  expect(onApply).not.toHaveBeenCalled();
  unmount();
  expect(opener).toHaveFocus();
  opener.remove();
});

it('blocks changes, Apply and dismissal while a batch is busy', () => {
  const { onApply, onClose } = setup(true);
  for (const button of screen.getAllByRole('button')) expect(button).toBeDisabled();
  expect(screen.getByRole('slider')).toBeDisabled();
  fireEvent.submit(screen.getByRole('button', { name: 'Apply display' }).closest('form')!);
  fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }));
  expect(onApply).not.toHaveBeenCalled();
  expect(onClose).not.toHaveBeenCalled();
});
