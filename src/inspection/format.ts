import { byteValue } from './treemap';

export function displayBytes(value: string | null): string {
  if (value === null) return 'Unknown';
  try {
    const bytes = byteValue(value);
    const units = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    let unit = 0;
    let divisor = 1n;
    while (unit < units.length - 1 && bytes >= divisor * 1000n) {
      divisor *= 1000n;
      unit += 1;
    }
    // Round with integer arithmetic so retained u128 counts stay lossless internally.
    const precision = unit === 0 || bytes >= divisor * 100n ? 1n : 10n;
    let rounded = (bytes * precision + divisor / 2n) / divisor;
    if (rounded >= 1000n * precision && unit < units.length - 1) {
      unit += 1;
      rounded = precision;
    }
    const text =
      precision === 1n || rounded % 10n === 0n
        ? String(rounded / precision)
        : `${rounded / 10n}.${rounded % 10n}`;
    return `${text} ${units[unit]}`;
  } catch {
    return 'Invalid retained byte count';
  }
}
