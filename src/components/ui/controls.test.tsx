import { createRef } from 'react';
import { fireEvent, render, screen } from '../../test/render';
import { expect, it, vi } from 'vitest';
import { Button, Input, Select, Textarea } from './controls';

it('preserves native submission and lossless form values across Chakra controls', () => {
  const submitted = vi.fn();
  render(
    <form
      onSubmit={(event) => {
        event.preventDefault();
        submitted(Object.fromEntries(new FormData(event.currentTarget)));
      }}
    >
      <label>
        Name
        <Input name="name" defaultValue="港, Family" size={24} required />
      </label>
      <label>
        Mode
        <Select name="mode" defaultValue="referenced">
          <option value="referenced">Read only</option>
        </Select>
      </label>
      <label>
        Caption
        <Textarea name="caption" defaultValue={'Line one\n<script>data</script>'} />
      </label>
      <Button>Save</Button>
    </form>,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect(submitted).toHaveBeenCalledWith({
    name: '港, Family',
    mode: 'referenced',
    caption: 'Line one\n<script>data</script>',
  });
  expect(screen.getByLabelText('Name')).toHaveAttribute('size', '24');
  expect(document.querySelector('script')).toBeNull();
});

it('keeps non-submit actions, native refs and disabled approvals intact', () => {
  const submitted = vi.fn((event) => event.preventDefault());
  const approved = vi.fn();
  const input = createRef<HTMLInputElement>();
  render(
    <form onSubmit={submitted}>
      <label>
        Search
        <Input ref={input} />
      </label>
      <Button type="button" onClick={() => input.current?.focus()}>
        Focus search
      </Button>
      <Button disabled onClick={approved}>
        Approve
      </Button>
      <fieldset disabled>
        <Button onClick={approved}>Unavailable approval</Button>
      </fieldset>
    </form>,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Focus search' }));
  expect(input.current).toBe(screen.getByLabelText('Search'));
  expect(input.current).toHaveFocus();
  expect(screen.getByRole('button', { name: 'Approve' })).toBeDisabled();
  expect(screen.getByRole('button', { name: 'Unavailable approval' })).toBeDisabled();
  fireEvent.click(screen.getByRole('button', { name: 'Approve' }));
  expect(approved).not.toHaveBeenCalled();
  expect(submitted).not.toHaveBeenCalled();
});
