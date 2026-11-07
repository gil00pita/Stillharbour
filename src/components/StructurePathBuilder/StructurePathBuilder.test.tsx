import { useState } from 'react';
import { afterEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '../../test/render';
import { StructurePathBuilder } from '@/components/StructurePathBuilder';
function Builder({
  disabled = false,
  initial = '{original_stem}.{ext}',
}: {
  disabled?: boolean;
  initial?: string;
}) {
  const [pattern, setPattern] = useState(initial);
  return (
    <>
      <output aria-label="Native rule pattern">{pattern}</output>
      <StructurePathBuilder pattern={pattern} disabled={disabled} onChange={setPattern} />
    </>
  );
}
afterEach(() => vi.restoreAllMocks());
it('builds a real ordered path with predefined chips, search and keyboard entry', () => {
  render(<Builder />);
  const input = screen.getByRole('combobox', { name: 'Folder levels' });
  fireEvent.change(input, { target: { value: 'Year' } });
  fireEvent.keyDown(input, { key: 'Enter' });
  fireEvent.change(input, { target: { value: 'Country' } });
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{capture:%Y}/{country}/{original_stem}.{ext}',
  );
  expect(
    screen.queryByRole('button', { name: /^Move .* (earlier|later)$/ }),
  ).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Reorder Country level 2' }));
  fireEvent.click(screen.getByRole('button', { name: 'Reorder Year level 1' }));
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{country}/{capture:%Y}/{original_stem}.{ext}',
  );
  fireEvent.click(screen.getByRole('button', { name: 'Remove Country level 1' }));
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{capture:%Y}/{original_stem}.{ext}',
  );
  fireEvent.keyDown(input, { key: 'Backspace' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent('{original_stem}.{ext}');
});
it('allows explicit literal folder levels without accepting path injection', () => {
  render(<Builder />);
  const input = screen.getByRole('combobox', { name: 'Folder levels' });
  fireEvent.change(input, { target: { value: 'Family photos' } });
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    'Family photos/{original_stem}.{ext}',
  );
  fireEvent.change(input, { target: { value: '../escape' } });
  expect(
    within(screen.getByRole('listbox', { name: 'Available folder parts' })).queryAllByRole(
      'option',
    ),
  ).toHaveLength(0);
  fireEvent.keyDown(input, { key: 'Enter' });
  expect(screen.getByLabelText('Native rule pattern')).not.toHaveTextContent('escape');
});
it('disables every editing action during preview creation', () => {
  render(<Builder disabled initial="{capture:%Y}/{country}/{original_stem}.{ext}" />);
  expect(screen.getByRole('combobox', { name: 'Folder levels' })).toBeDisabled();
  expect(screen.getByRole('textbox', { name: 'Filename pattern' })).toBeDisabled();
  for (const option of screen.getAllByRole('option')) expect(option).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Reorder Country level 2' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Remove Country level 2' })).toBeDisabled();
});

const initial = '{capture:%Y}/{country}/{city}/{original_stem}.{ext}';
function layoutChips() {
  const handles = screen.getAllByRole('button', { name: /^Reorder / });
  const scope = handles[0]!.closest('.structure-path-input')!;
  vi.spyOn(scope, 'getBoundingClientRect').mockReturnValue(new DOMRect(0, 0, 200, 120));
  handles.forEach((handle, index) => {
    vi.spyOn(handle, 'getBoundingClientRect').mockReturnValue(
      new DOMRect(index === 1 ? 100 : 0, index === 2 ? 60 : 0, 80, 40),
    );
  });
  return handles[0]!;
}
function startDrag(handle: HTMLElement, pointerId = 1) {
  fireEvent.pointerDown(handle, {
    pointerId,
    button: 0,
    isPrimary: true,
    clientX: 40,
    clientY: 20,
  });
  fireEvent.pointerMove(handle, { pointerId, clientX: 40, clientY: 80 });
}
it('drags across wrapped rows, previews the drop position and commits once on release', () => {
  render(<Builder initial={initial} />);
  const handle = layoutChips();
  startDrag(handle);
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(initial);
  expect(
    screen.getByRole('button', { name: 'Reorder City level 3' }).closest('li'),
  ).toHaveAttribute('data-drop', 'after');
  expect(document.querySelector('[data-folder-drag-preview]')).toBeInTheDocument();
  fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40, clientY: 80 });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{country}/{city}/{capture:%Y}/{original_stem}.{ext}',
  );
  expect(screen.getByRole('button', { name: 'Reorder Year level 3' })).toHaveFocus();
  expect(screen.getByRole('status', { name: 'Folder order' })).toHaveTextContent(
    'Year placed at level 3 of 3.',
  );
  expect(document.querySelector('[data-folder-drag-preview]')).not.toBeInTheDocument();
  // The compatibility click following a drag must not pick the chip up again.
  fireEvent.click(screen.getByRole('button', { name: 'Reorder Year level 3' }));
  expect(screen.getByRole('button', { name: 'Reorder Year level 3' })).toHaveAttribute(
    'aria-pressed',
    'false',
  );
});
it.each(['outside', 'escape', 'pointercancel', 'lostcapture', 'blur'])(
  'cancels a drag on %s without changing the path',
  (reason) => {
    render(<Builder initial={initial} />);
    const handle = layoutChips();
    startDrag(handle);
    if (reason === 'escape') fireEvent.keyDown(handle, { key: 'Escape' });
    if (reason === 'pointercancel') fireEvent.pointerCancel(handle, { pointerId: 1 });
    if (reason === 'lostcapture') fireEvent.lostPointerCapture(handle, { pointerId: 1 });
    if (reason === 'blur') fireEvent(window, new Event('blur'));
    fireEvent.pointerUp(handle, {
      pointerId: 1,
      clientX: reason === 'outside' ? 300 : 40,
      clientY: 80,
    });
    expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(initial);
    expect(document.querySelector('[data-folder-drag-preview]')).not.toBeInTheDocument();
  },
);
it('supports bounded keyboard pickup, destination selection, cancellation and focus restoration', () => {
  render(<Builder initial={initial} />);
  const handle = screen.getByRole('button', { name: 'Reorder Year level 1' });
  fireEvent.keyDown(handle, { key: ' ' });
  fireEvent.keyDown(handle, { key: 'Home' });
  fireEvent.keyDown(handle, { key: 'ArrowLeft' });
  expect(screen.getByRole('status', { name: 'Folder order' })).toHaveTextContent(
    'Destination level 1 of 3',
  );
  fireEvent.keyDown(handle, { key: 'End' });
  fireEvent.keyDown(handle, { key: 'ArrowRight' });
  expect(screen.getByRole('status', { name: 'Folder order' })).toHaveTextContent(
    'Destination level 3 of 3',
  );
  fireEvent.keyDown(handle, { key: 'ArrowLeft' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(initial);
  fireEvent.keyDown(handle, { key: 'Enter' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{country}/{capture:%Y}/{city}/{original_stem}.{ext}',
  );
  const moved = screen.getByRole('button', { name: 'Reorder Year level 2' });
  expect(moved).toHaveFocus();
  fireEvent.keyDown(moved, { key: ' ' });
  fireEvent.keyDown(moved, { key: 'Home' });
  fireEvent.keyDown(moved, { key: 'Escape' });
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{country}/{capture:%Y}/{city}/{original_stem}.{ext}',
  );
});
it('cancels stale gestures when the rule changes or editing becomes disabled', () => {
  const onChange = vi.fn();
  const view = render(
    <StructurePathBuilder pattern={initial} disabled={false} onChange={onChange} />,
  );
  let handle = layoutChips();
  startDrag(handle);
  view.rerender(<StructurePathBuilder pattern={initial} disabled onChange={onChange} />);
  fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40, clientY: 80 });
  expect(onChange).not.toHaveBeenCalled();
  view.rerender(<StructurePathBuilder pattern={initial} disabled={false} onChange={onChange} />);
  handle = layoutChips();
  startDrag(handle);
  view.rerender(
    <StructurePathBuilder
      pattern="{country}/{original_stem}.{ext}"
      disabled={false}
      onChange={onChange}
    />,
  );
  fireEvent.pointerUp(handle, { pointerId: 1, clientX: 40, clientY: 80 });
  expect(onChange).not.toHaveBeenCalled();
});
it('preserves duplicate levels and the filename while using click-to-place', () => {
  render(<Builder initial="{country}/{capture:%Y}/{country}/custom_{original_stem}.{ext}" />);
  fireEvent.click(screen.getByRole('button', { name: 'Reorder Country level 3' }));
  fireEvent.click(screen.getByRole('button', { name: 'Reorder Year level 2' }));
  expect(screen.getByLabelText('Native rule pattern')).toHaveTextContent(
    '{country}/{country}/{capture:%Y}/custom_{original_stem}.{ext}',
  );
  expect(screen.getAllByRole('button', { name: /^Reorder Country/ })).toHaveLength(2);
});
