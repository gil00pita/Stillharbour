import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../../test/render';
import { OrganizationTemplateFields } from './OrganizationTemplateFields';

it('retains a legacy saved event rule until the user chooses a date-based preset', async () => {
  const onChange = vi.fn();
  const pattern = '{event}/{capture:%Y-%m-%d}/{original_stem}.{ext}';
  render(
    <OrganizationTemplateFields
      template={{ pattern, event: 'Retained trip', missingDate: 'undated' }}
      busy={false}
      onChange={onChange}
    />,
  );
  expect(screen.getByRole('textbox', { name: 'Event name' })).toHaveValue('Retained trip');
  expect(screen.getByRole('textbox', { name: 'Path template' })).toHaveValue(pattern);
  expect(screen.getByRole('combobox', { name: 'Folder levels' })).toBeVisible();
  fireEvent.click(screen.getByRole('radio', { name: 'By date' }));
  await screen.findByRole('radio', { name: 'Date & time' });
  expect(onChange).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('radio', { name: 'Date & time' }));
  await waitFor(() =>
    expect(onChange).toHaveBeenCalledWith({
      pattern: '{capture:%Y-%m-%d}/{capture:%H%M%S}_{original_stem}.{ext}',
      event: '',
    }),
  );
});
