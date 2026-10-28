import type { DateBucket, MonthBucket } from '../bindings';
import { timelineDate } from './model';
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
          Math.max(1, periods.length - 1)) *
        Math.max(0, height - 28),
    }))
    .sort((a, b) => a.priority - b.priority || a.top - b.top);
  const placed: typeof candidates = [];
  for (const candidate of candidates) {
    if (placed.every((other) => Math.abs(other.top - candidate.top) >= 34)) placed.push(candidate);
  }
  return placed.sort((a, b) => a.top - b.top);
}
