import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within, waitFor } from '../../test/render';
import { SegmentedControl } from '@/components/SegmentedControl';
const choices = [
  { value: 'preserve', label: 'Preserve' },
  { value: 'upper', label: 'UPPERCASE' },
];
it('exposes a labelled radio group and forwards only the selected typed choice', async () => {
  const onChange = vi.fn();
  const view = render(
    <SegmentedControl
      label="Filename case"
      value="preserve"
      choices={choices}
      disabled={false}
      onChange={onChange}
    />,
  );
  const group = screen.getByRole('radiogroup', { name: 'Filename case' });
  expect(within(group).getByRole('radio', { name: 'Preserve' })).toBeChecked();
  fireEvent.click(within(group).getByRole('radio', { name: 'UPPERCASE' }));
  await waitFor(() => expect(onChange).toHaveBeenCalledWith('upper'));
  view.rerender(
    <SegmentedControl
      label="Filename case"
      value="upper"
      choices={choices}
      disabled
      onChange={onChange}
    />,
  );
  onChange.mockClear();
  for (const radio of within(group).getAllByRole('radio')) expect(radio).toBeDisabled();
  fireEvent.click(within(group).getByRole('radio', { name: 'Preserve' }));
  expect(onChange).not.toHaveBeenCalled();
});
