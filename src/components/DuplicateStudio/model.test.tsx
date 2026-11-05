import { expect, it } from 'vitest';
import { duplicateBytes, suggestKeepers } from './model';

import { group, member } from './fixtures';
it('keeps ties and unknown evidence instead of silently picking a removable copy', () => {
  const rows = [member('a'), member('b')];
  expect(suggestKeepers(rows, 'resolution')).toEqual(new Set(['a', 'b']));
  expect(suggestKeepers([rows[0]!, member('b', { width: null })], 'resolution')).toEqual(
    new Set(['a', 'b']),
  );
  expect(suggestKeepers(rows, 'oldest')).toEqual(new Set());
  expect(
    suggestKeepers([member('raw', { name: 'Original.CR3' }), member('export')], 'raw'),
  ).toEqual(new Set(['raw']));
  expect(
    suggestKeepers(
      [member('a', { bytes: '9007199254740992' }), member('b', { bytes: '9007199254740993' })],
      'largest',
    ),
  ).toEqual(new Set(['b']));
});
it('never turns candidate matches, linked paths, incomplete pages or offline entries into recoverable bytes', () => {
  const rows = [member('a'), member('b')];
  expect(duplicateBytes(group, rows)).toBe('100');
  expect(duplicateBytes({ ...group, evidence: 'normalized_pixels' }, rows)).toBeNull();
  expect(duplicateBytes({ ...group, membersTruncated: true }, rows)).toBeNull();
  expect(duplicateBytes(group, [rows[0]!])).toBeNull();
  expect(duplicateBytes(group, [rows[0]!, member('b', { links: '2' })])).toBeNull();
  expect(duplicateBytes(group, [rows[0]!, member('b', { inventory: 'offline' })])).toBeNull();
});
