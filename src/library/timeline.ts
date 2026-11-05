import type { DateBucket, MonthBucket } from '../bindings';
import { PHOTO_GUTTER, timelineDate, type DateGrouping, type PhotoRow } from './model';
export function periodLabel(period?: string) {
  if (!period || period === 'Undated') return 'Undated';
  return period.length === 7 ? timelineDate(`${period}-01`).label : period;
}

export function timelineTarget(periods: MonthBucket[], value: number) {
  const total = periods.reduce((sum, item) => sum + item.count, 0);
  const index = Math.max(0, Math.min(Math.max(0, total - 1), Math.round(value)));
  let start = 0;
  for (const item of periods) {
    if (index < start + item.count)
      return {
        period: item.period,
        index,
        progress: item.count <= 1 ? 0 : (index - start) / (item.count - 1),
      };
    start += item.count;
  }
  return { period: undefined, index: 0, progress: 0 };
}

export function timelineScrollState(periods: MonthBucket[], date: string, progress: number) {
  const current = timelineDate(date);
  const period = current.year === 'Undated' ? 'Undated' : date.slice(0, 7);
  let start = 0;
  let position = 0;
  for (const item of periods) {
    if (item.period === period)
      position = start + Math.max(0, Math.min(1, progress)) * Math.max(0, item.count - 1);
    start += item.count;
  }
  return { current, period, maximum: Math.max(0, start - 1), position };
}

export function timelinePositionAt(
  y: number,
  bounds: { top: number; height: number },
  maximum: number,
  fallback: number,
) {
  if (!Number.isFinite(y) || bounds.height <= 0) return fallback;
  return Math.max(0, Math.min(maximum, ((y - bounds.top) / bounds.height) * maximum));
}

export function timelineKeyPosition(key: string, value: number, maximum: number) {
  if (key === 'Home') return 0;
  if (key === 'End') return maximum;
  const steps: Record<string, number> = {
    ArrowDown: 1,
    ArrowRight: 1,
    ArrowUp: -1,
    ArrowLeft: -1,
    PageDown: Math.max(1, Math.round(maximum / 10)),
    PageUp: -Math.max(1, Math.round(maximum / 10)),
  };
  return Object.hasOwn(steps, key) ? Math.max(0, Math.min(maximum, value + steps[key]!)) : null;
}

// The reference places years by the ordinal of their first photo, not time gaps.
export function timelineYears(
  buckets: DateBucket[],
  _periods: MonthBucket[],
  _current: string,
  height: number,
) {
  const maximum = Math.max(1, buckets.reduce((sum, bucket) => sum + bucket.count, 0) - 1);
  let start = 0;
  let previous = -Infinity;
  return buckets.flatMap((bucket) => {
    const top = (start / maximum) * Math.max(0, height - 48);
    start += bucket.count;
    const visible = top > previous + 20;
    previous = top;
    return visible ? [{ bucket, top }] : [];
  });
}

// Upstream dots are 4px fillers every 12px between year labels, not month ticks.
export function timelineTicks(periods: MonthBucket[], height: number) {
  const maximum = Math.max(1, periods.reduce((sum, period) => sum + period.count, 0) - 1);
  const travel = Math.max(0, height - 48);
  const starts: number[] = [];
  let count = 0;
  let year = '';
  for (const period of periods) {
    const next = period.period.slice(0, 4);
    if (next !== year) {
      starts.push((count / maximum) * travel);
      year = next;
    }
    count += period.count;
  }
  const dots: { period: string; position: number }[] = [];
  for (let i = 1; i < starts.length; i++) {
    const first = starts[i - 1]!;
    const last = starts[i]!;
    const amount = Math.floor((last - 25 - first) / 12);
    for (let dot = 0; dot < amount && dots.length < 80; dot++)
      dots.push({
        period: `dot-${i}-${dot}`,
        position: (first + 25 + dot * 12) / Math.max(1, travel),
      });
  }
  return dots;
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
