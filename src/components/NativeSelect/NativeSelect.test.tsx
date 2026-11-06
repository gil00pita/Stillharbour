import { Field } from '@chakra-ui/react';
import { createRef } from 'react';
import { expect, it } from 'vitest';
import { render, screen, waitFor } from '../../test/render';
import { NativeSelect } from '@/components/NativeSelect';

it('retains one native select, its ref, numeric size and multiple form values', () => {
  const ref = createRef<HTMLSelectElement>();
  render(
    <form aria-label="Selection">
      <Field.Root>
        <Field.Label>Libraries</Field.Label>
        <NativeSelect ref={ref} name="libraries" size={3} multiple defaultValue={['a', 'b']}>
          <option value="a">Family</option>
          <option value="b">Archive</option>
        </NativeSelect>
      </Field.Root>
    </form>,
  );
  const select = screen.getByRole('listbox', { name: 'Libraries' });
  expect(ref.current).toBe(select);
  expect(select).toHaveAttribute('size', '3');
  expect(select.parentElement).toHaveAttribute('data-scope', 'field');
  expect(new FormData(screen.getByRole('form') as HTMLFormElement).getAll('libraries')).toEqual([
    'a',
    'b',
  ]);
});

it('inherits disabled, required and invalid field state with accessible feedback', async () => {
  render(
    <Field.Root disabled required invalid>
      <Field.Label>Destination</Field.Label>
      <NativeSelect defaultValue="">
        <option value="">Choose a destination</option>
      </NativeSelect>
      <Field.HelperText>Only approved destinations are available.</Field.HelperText>
      <Field.ErrorText>A destination is required.</Field.ErrorText>
    </Field.Root>,
  );
  const select = screen.getByRole('combobox', { name: 'Destination' });
  expect(select).toBeDisabled();
  expect(select).toBeRequired();
  expect(select).toHaveAttribute('aria-invalid', 'true');
  await waitFor(() => {
    expect(select).toHaveAccessibleDescription('Only approved destinations are available.');
    expect(select).toHaveAccessibleErrorMessage('A destination is required.');
  });
});
