import { expect, it } from 'vitest';
import { timelineKeyPosition, timelinePositionAt, timelineTicks } from './timeline';

it('keeps keyboard movement bounded and leaves unrelated keys to the application', () => {
  expect(timelineKeyPosition('Home', 2.5, 3)).toBe(0);
  expect(timelineKeyPosition('End', 0, 3)).toBe(3);
  expect(timelineKeyPosition('ArrowDown', 2.99, 3)).toBe(3);
  expect(timelineKeyPosition('ArrowUp', 0.01, 3)).toBe(0);
  expect(timelineKeyPosition('PageDown', 0.25, 3)).toBe(1.25);
  expect(timelineKeyPosition('PageUp', 0.25, 3)).toBe(0);
  expect(timelineKeyPosition('Tab', 0, 3)).toBeNull();
  expect(timelineKeyPosition('toString', 0, 3)).toBeNull();
});

it('clamps outside pointer positions and retains the draft for missing geometry', () => {
  const bounds = { top: 14, height: 300 };
  expect(timelinePositionAt(164, bounds, 3, 0)).toBe(1.5);
  expect(timelinePositionAt(-50, bounds, 3, 0)).toBe(0);
  expect(timelinePositionAt(500, bounds, 3, 0)).toBe(3);
  expect(timelinePositionAt(Number.NaN, bounds, 3, 1)).toBe(1);
  expect(timelinePositionAt(164, { top: 14, height: 0 }, 3, 1)).toBe(1);
});

it('places year labels and filler dots by photo rank instead of equal month slots', () => {
  const periods = [
    { period: '2026-03', count: 900 },
    { period: '2025-01', count: 100 },
  ];
  const ticks = timelineTicks(periods, 400);
  expect(ticks.length).toBeGreaterThan(20);
  expect(ticks.length).toBeLessThanOrEqual(80);
  for (let i = 1; i < ticks.length; i++)
    expect(ticks[i]!.position).toBeGreaterThan(ticks[i - 1]!.position);
  expect(ticks[0]!.position * 352).toBeCloseTo(25);
  expect((ticks[1]!.position - ticks[0]!.position) * 352).toBeCloseTo(12);
});
it('does not invent dots for an empty or single-year collection', () => {
  expect(timelineTicks([], 400)).toEqual([]);
  expect(timelineTicks([{ period: '2026-03', count: 400 }], 400)).toEqual([]);
  expect(
    timelineTicks(
      [
        { period: '2026-03', count: 400 },
        { period: 'Undated', count: 1 },
      ],
      20,
    ),
  ).toEqual([]);
});
