import { expect, it } from 'vitest';
import { displayBytes } from './format';

it('uses decimal sizes, trims zeros and carries rounding at unit boundaries', () => {
  for (const [bytes, label] of [
    ['0', '0 B'],
    ['824', '824 B'],
    ['24000', '24 KB'],
    ['680000', '680 KB'],
    ['12400000', '12.4 MB'],
    ['9600000000', '9.6 GB'],
    ['1200000000000', '1.2 TB'],
    ['999999999', '1 GB'],
    ['999999', '1 MB'],
    ['999', '999 B'],
  ] as const)
    expect(displayBytes(bytes)).toBe(label);
  expect(displayBytes(null)).toBe('Unknown');
  expect(displayBytes('-1')).toBe('Invalid retained byte count');
  expect(displayBytes((1n << 100n).toString())).not.toContain('e+');
});
