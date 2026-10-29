import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FileSizeField } from './FileSizeField';

function setup(initialBytes: string | null = null, disabled = false) {
  const onChange = vi.fn();
  render(
    <FileSizeField
      name="minBytes"
      label="Minimum size"
      initialBytes={initialBytes}
      disabled={disabled}
      onChange={onChange}
    />,
  );
  return {
    onChange,
    input: screen.getByRole('textbox', { name: 'Minimum size' }),
    unit: screen.getByRole('combobox', { name: 'Minimum size unit' }),
  };
}

describe('file size units', () => {
  it('converts decimal KB, MB and GB to exact bytes without rounding', () => {
    const { input, unit, onChange } = setup();
    fireEvent.change(input, { target: { value: '1.5' } });
    expect(onChange).toHaveBeenLastCalledWith('1500000');
    fireEvent.change(unit, { target: { value: 'KB' } });
    expect(input).toHaveValue('1.5');
    expect(onChange).toHaveBeenLastCalledWith('1500');
    fireEvent.change(unit, { target: { value: 'GB' } });
    expect(onChange).toHaveBeenLastCalledWith('1500000000');
  });
  it('shows arbitrary saved byte counts exactly, including values beyond Number precision', () => {
    const { input, unit, onChange } = setup('9007199254740993');
    expect(unit).toHaveValue('GB');
    expect(input).toHaveValue('9007199.254740993');
    fireEvent.change(input, { target: { value: '9007199.2547409930' } });
    expect(onChange).toHaveBeenLastCalledWith('9007199254740993');
  });
  it('distinguishes an unlimited blank from a zero-byte threshold', () => {
    const { input, onChange } = setup();
    fireEvent.change(input, { target: { value: '0' } });
    expect(onChange).toHaveBeenLastCalledWith('0');
    fireEvent.change(input, { target: { value: '' } });
    expect(onChange).toHaveBeenLastCalledWith(null);
  });
  it.each(['-1', '1e3', 'Infinity', '0.0000001', '18446744073709551616'])(
    'rejects %s without emitting a changed threshold',
    (value) => {
      const { input, onChange } = setup();
      fireEvent.change(input, { target: { value } });
      expect(input).toBeInvalid();
      expect(input).toHaveAccessibleDescription();
      expect(onChange).not.toHaveBeenCalled();
    },
  );
  it('allows correcting a fractional-byte input by choosing a larger unit', () => {
    const { input, unit, onChange } = setup();
    fireEvent.change(unit, { target: { value: 'KB' } });
    onChange.mockClear();
    fireEvent.change(input, { target: { value: '.000001' } });
    expect(input).toBeInvalid();
    fireEvent.change(unit, { target: { value: 'MB' } });
    expect(input).toBeValid();
    expect(onChange).toHaveBeenLastCalledWith('1');
  });
  it('disables both controls while the form is busy', () => {
    const { input, unit } = setup('1500', true);
    expect(input).toHaveValue('1.5');
    expect(unit).toHaveValue('KB');
    expect(input).toBeDisabled();
    expect(unit).toBeDisabled();
  });
});
