import { byteValue } from './treemap';

export function displayBytes(value: string | null): string {
  if (value === null) return 'Unknown';
  try {
    return `${byteValue(value).toLocaleString()} B`;
  } catch {
    return 'Invalid retained byte count';
  }
}
