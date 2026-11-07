import { useState } from 'react';
import { expect, it } from 'vitest';
import { fireEvent, render, screen, within } from '../../test/render';
import { StructurePathBuilder } from '@/components/StructurePathBuilder';
function Builder({ disabled = false }: { disabled?: boolean }) {
  const [pattern, setPattern] = useState('{original_stem}.{ext}');
  return (
    <>
      <output aria-label="Native rule pattern">{pattern}</output>
      <StructurePathBuilder pattern={pattern} disabled={disabled} onChange={setPattern} />
    </>
  );
}
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
  fireEvent.click(screen.getByRole('button', { name: 'Move Country level 2 earlier' }));
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
  render(<Builder disabled />);
  expect(screen.getByRole('combobox', { name: 'Folder levels' })).toBeDisabled();
  expect(screen.getByRole('textbox', { name: 'Filename pattern' })).toBeDisabled();
  for (const option of screen.getAllByRole('option')) expect(option).toBeDisabled();
});
