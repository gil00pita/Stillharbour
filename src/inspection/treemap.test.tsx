import { expect, it } from 'vitest';
import { byteValue, mapTiles, remainingBytes } from './treemap';

it('plots the same lossless scope weights without gaps or overlap', () => {
  const tiles = mapTiles([
    { id: 'a', label: '100-byte entry', bytes: '100' },
    { id: 'b', label: '200-byte entry', bytes: '200' },
    { id: 'c', label: '300-byte entry', bytes: '300' },
  ]);
  expect(tiles).toHaveLength(3);
  expect(tiles.reduce((area, tile) => area + tile.width * tile.height, 0)).toBeCloseTo(5000);
  for (const tile of tiles) {
    expect((tile.width * tile.height) / 5000).toBeCloseTo(Number(tile.bytes) / 600);
    expect(tile.x).toBeGreaterThanOrEqual(0);
    expect(tile.y).toBeGreaterThanOrEqual(0);
    expect(tile.x + tile.width).toBeLessThanOrEqual(100.00000001);
    expect(tile.y + tile.height).toBeLessThanOrEqual(50.00000001);
  }
  for (const [index, a] of tiles.entries())
    for (const b of tiles.slice(index + 1))
      expect(
        Math.max(0, Math.min(a.x + a.width, b.x + b.width) - Math.max(a.x, b.x)) *
          Math.max(0, Math.min(a.y + a.height, b.y + b.height) - Math.max(a.y, b.y)),
      ).toBeCloseTo(0);
});

it('keeps byte counts above JavaScript integer precision and accounts for unplotted remainder', () => {
  const huge = (1n << 90n).toString();
  expect(byteValue(huge)).toBe(1n << 90n);
  const tiles = mapTiles([
    { id: 'huge', label: '__proto__', bytes: huge },
    { id: 'other', label: '<script>', bytes: huge },
  ]);
  const first = tiles[0];
  if (!first) throw new Error('Expected a measured map tile.');
  expect(first.width * first.height).toBeCloseTo(2500);
  expect(remainingBytes('1000', ['100', '200', '300'])).toBe('400');
  expect(remainingBytes(huge, ['1'])).toBe(((1n << 90n) - 1n).toString());
  expect(() => remainingBytes('1', ['2'])).toThrow('does not balance');
});

it('rejects malformed identities/metrics and does not invent area for zero bytes', () => {
  for (const bad of ['-1', '1.5', '', '00', '1e9', (1n << 128n).toString()])
    expect(() => byteValue(bad)).toThrow();
  expect(mapTiles([{ id: 'empty', label: 'Empty file', bytes: '0' }])).toEqual([]);
  expect(() =>
    mapTiles([
      { id: 'a', label: 'a', bytes: '1' },
      { id: 'a', label: 'b', bytes: '2' },
    ]),
  ).toThrow('Repeated');
  expect(() => mapTiles([], 0, 10)).toThrow('boundary');
});
