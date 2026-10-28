import { describe, expect, it } from 'vitest';
import { PHOTO_GUTTER, DATE_HEADING_HEIGHT, photoRows, timelineDate, dayLabel } from './model';
import { fixtureAsset } from './__tests__/fixtures';

describe('aspect-preserving chronological rows', () => {
  it.each([240, 700, 1300])('preserves composition with narrow gutters at %spx', (width) => {
    const assets = Array.from({ length: 17 }, (_, index) => ({
      ...fixtureAsset(`${index}`),
      width: index % 2 ? 360 : 600,
      height: index % 2 ? 480 : 380,
    }));
    const rows = photoRows(assets, width, 'gallery');
    expect(rows.flatMap((row) => row.assets.map((asset) => asset.id))).toEqual(
      assets.map((asset) => asset.id),
    );
    for (const row of rows) {
      expect(
        row.widths.reduce((a, b) => a + b, 0) + PHOTO_GUTTER * (row.assets.length - 1),
      ).toBeLessThanOrEqual(width + 0.001);
      row.assets.forEach((asset, index) =>
        expect(row.widths[index]! / row.imageHeight).toBeCloseTo(asset.width! / asset.height!, 6),
      );
      expect(row.height - row.imageHeight).toBe(row.headings.length ? DATE_HEADING_HEIGHT : 0);
      expect(row.height).toBeGreaterThan(0);
      expect(row.imageHeight).toBeLessThanOrEqual(275);
      expect(row.widths.every((value) => value > 0 && Number.isFinite(value))).toBe(true);
    }
  });
  it('bounds missing or extreme dimensions and a single final image', () => {
    const assets = [
      { ...fixtureAsset('unknown'), width: null, height: null },
      { ...fixtureAsset('extreme'), width: 999999, height: 1 },
    ];
    const rows = photoRows(assets, 700, 'gallery');
    expect(rows.every((row) => row.imageHeight <= 220 && Number.isFinite(row.height))).toBe(true);
    expect(photoRows([assets[0]!], 700, 'gallery')[0]?.widths[0]).toBeLessThan(700);
    expect(photoRows([], 700, 'gallery')).toEqual([]);
  });
  it('keeps Grid captions/gaps and Text list geometry independent of Gallery', () => {
    const assets = Array.from({ length: 6 }, (_, index) => fixtureAsset(`${index}`));
    const grid = photoRows(assets, 700, 'grid')[0]!;
    expect(
      grid.widths.reduce((a, b) => a + b, 0) + PHOTO_GUTTER * (grid.assets.length - 1),
    ).toBeCloseTo(700);
    expect(grid.height).toBeCloseTo(grid.widths[0]! + 42);
    expect(photoRows(assets, 700, 'list')).toHaveLength(6);
    expect(photoRows(assets, 700, 'list')[0]?.height).toBe(54);
  });
});

describe('timeline capture-local dates', () => {
  it('retains pre-1970 and unknown-offset month/year without timezone conversion', () => {
    const date = timelineDate('1964-03-01T00:00:00');
    expect(date.year).toBe('1964');
    expect(date.label).toContain('1964');
    expect(date.position).toBe(75);
    expect(timelineDate('2026-12-31T23:59:59-12:00').year).toBe('2026');
    expect(timelineDate('2026-12-31T23:59:59-12:00').position).toBe(0);
  });
  it.each(['', 'not a date', '2026-99-01'])(
    'keeps %s undated rather than inventing a date',
    (value) => {
      expect(timelineDate(value)).toEqual({ year: 'Undated', label: 'Undated', position: 50 });
    },
  );
});

it('places sparse consecutive dates beside one another without reordering photos', () => {
  const assets = [fixtureAsset('one', '2026-03-02'), fixtureAsset('two', '2026-03-01')];
  const rows = photoRows(assets, 1300, 'gallery');
  expect(rows).toHaveLength(1);
  expect(rows[0]?.headings.map((heading) => heading.date)).toEqual(['2026-03-02', '2026-03-01']);
  expect(rows[0]?.assets.map((asset) => asset.id)).toEqual(['one', 'two']);
});
it('distinguishes capture dates, recently added and unknown dates without changing metadata', () => {
  const asset = fixtureAsset('one', '1964-07-01');
  expect(photoRows([asset], 700, 'gallery', 220, 'added')[0]?.headings[0]?.date).toBe('2026-10-27');
  expect(asset.date).toBe('1964-07-01');
  expect(dayLabel('2026-10-27', new Date(2026, 9, 27, 10))).toBe('Today');
  expect(dayLabel('2026-10-26', new Date(2026, 9, 27, 10))).toBe('Yesterday');
  expect(dayLabel('Undated')).toBe('Unknown date');
});

it('balances a whole sequence instead of making a panorama share an already crowded row', () => {
  const assets = [1.5, 1.5, 1.5, 8, 1.5, 1.5, 1.5].map((ratio, index) => ({
    ...fixtureAsset(`${index}`),
    width: ratio * 1000,
    height: 1000,
  }));
  const rows = photoRows(assets, 1000, 'gallery');
  const panorama = rows.find((row) => row.assets.some((asset) => asset.id === '3'))!;
  expect(panorama.assets.map((asset) => asset.id)).toEqual(['3']);
  expect(panorama.imageHeight).toBeCloseTo(125);
  expect(rows.flatMap((row) => row.assets.map((asset) => asset.id))).toEqual(
    assets.map((asset) => asset.id),
  );
});
it('balances a dense day without leaving a tiny final orphan', () => {
  const assets = Array.from({ length: 8 }, (_, index) => fixtureAsset(`${index}`));
  const rows = photoRows(assets, 1000, 'gallery');
  expect(rows.map((row) => row.assets.length)).toEqual([4, 4]);
  expect(rows.every((row) => Math.abs(row.imageHeight - 220) < 60)).toBe(true);
});
it('keeps bounded, deterministic geometry across the full retained feed and extreme metadata', () => {
  const assets = Array.from({ length: 720 }, (_, index) => ({
    ...fixtureAsset(`${index}`, index % 3 ? '1964-07-01' : ''),
    width: [1, 20000, null, Infinity, 1500][index % 5]!,
    height: 1000,
  }));
  for (const width of [160, 660, 1300]) {
    const rows = photoRows(assets, width, 'gallery');
    expect(rows.flatMap((row) => row.assets)).toEqual(assets);
    expect(
      rows.every((row) => row.assets.length <= 64 && row.imageHeight > 0 && row.imageHeight <= 275),
    ).toBe(true);
    expect(
      rows.every(
        (row) =>
          row.widths.reduce((a, b) => a + b, 0) + PHOTO_GUTTER * (row.assets.length - 1) <=
          width + 0.001,
      ),
    ).toBe(true);
    expect(photoRows(assets, width, 'gallery')).toEqual(rows);
  }
});
it('keeps compact date captions fully named and preserves Today and unknown dates', () => {
  expect(dayLabel('1964-07-01', new Date(2026, 9, 28), true)).toContain('1964');
  expect(dayLabel('1964-07-01', new Date(2026, 9, 28), true).length).toBeLessThan(
    dayLabel('1964-07-01').length,
  );
  expect(dayLabel('2026-10-28', new Date(2026, 9, 28), true)).toBe('Today');
  expect(dayLabel('Undated', new Date(), true)).toBe('Unknown date');
});
