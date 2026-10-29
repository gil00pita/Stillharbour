import type { DateBucket, MonthBucket } from '../bindings';
import { PHOTO_GUTTER, timelineDate, type DateGrouping, type PhotoRow } from './model';
export function periodLabel(period?: string) {
  if (!period || period === 'Undated') return 'Undated';
  return period.length === 7 ? timelineDate(`${period}-01`).label : period;
}
// Place real year labels at their month position and leave room for their hit areas.
export function timelineYears(
  buckets: DateBucket[],
  periods: MonthBucket[],
  current: string,
  height: number,
) {
  const candidates = buckets
    .map((bucket, index) => ({
      bucket,
      priority: bucket.year === current ? 0 : index === 0 || index === buckets.length - 1 ? 1 : 2,
      top:
        (Math.max(
          0,
          periods.findIndex((period) => period.period.startsWith(bucket.year)),
        ) /
          Math.max(1, periods.length)) *
        Math.max(0, height - 28),
    }))
    .sort((a, b) => a.priority - b.priority || a.top - b.top);
  const placed: typeof candidates = [];
  for (const candidate of candidates) {
    if (placed.every((other) => Math.abs(other.top - candidate.top) >= 34)) placed.push(candidate);
  }
  return placed.sort((a, b) => a.top - b.top);
}

export function timelinePeriod(date: string) {
  return date ? date.slice(0, 7) : 'Undated';
}

// Only retained row geometry is inspected (at most 720 summaries). A month
// outside this window still uses the existing indexed catalog seek.
export function timelineScrollRange(
  rows: PhotoRow[],
  period: string,
  grouping: DateGrouping,
  viewportHeight: number,
) {
  let offset = 0;
  let start: number | undefined;
  let end = 0;
  for (const row of rows) {
    if (
      row.assets.some(
        (asset) => timelinePeriod(grouping === 'added' ? asset.addedAt : asset.date) === period,
      )
    ) {
      start ??= offset;
      end = offset + row.height + PHOTO_GUTTER;
    }
    offset += row.height + PHOTO_GUTTER;
  }
  if (start === undefined) return null;
  const maximum = Math.max(0, offset - viewportHeight);
  return { start: Math.min(start, maximum), end: Math.min(end, maximum), maximum };
}
