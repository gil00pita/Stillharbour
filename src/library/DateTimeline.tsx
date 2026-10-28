import { useEffect, useRef, useState } from 'react';
import type { DateBucket } from '../bindings';
import { timelineDate } from './model';
import { periodLabel, timelineYears } from './timeline';

export function DateTimeline({
  buckets,
  date,
  scrolling,
  onYear,
}: {
  buckets: DateBucket[];
  date: string;
  scrolling: boolean;
  onYear: (period: string) => void;
}) {
  const rail = useRef<HTMLElement>(null);
  const [height, setHeight] = useState(400);
  const [draft, setDraft] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [focused, setFocused] = useState(false);
  const [hovered, setHovered] = useState<number | null>(null);
  const periods = buckets.flatMap((bucket) =>
    bucket.months.length ? bucket.months : [{ period: bucket.year, count: bucket.count }],
  );
  const current = timelineDate(date);
  const currentPeriod = current.year === 'Undated' ? 'Undated' : date.slice(0, 7);
  const activeIndex = Math.max(
    0,
    periods.findIndex((period) => period.period === currentPeriod),
  );
  const value =
    dragging || focused ? Math.min(draft, Math.max(0, periods.length - 1)) : activeIndex;
  const labelIndex = hovered ?? value;
  const period = periods[labelIndex]?.period;
  const label = periodLabel(period);
  useEffect(() => {
    const element = rail.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setHeight(entries[0]?.contentRect.height ?? 400),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const visibleYears = timelineYears(buckets, periods, current.year, height);
  function commit(index: number) {
    const selected = periods[Math.max(0, Math.min(periods.length - 1, index))];
    if (selected) onYear(selected.period);
  }
  return (
    <nav
      ref={rail}
      className={`date-rail ${scrolling || dragging || focused ? 'is-scrolling' : ''}`}
      aria-label="Photo timeline"
    >
      <div className="timeline-year-labels">
        {visibleYears.map(({ bucket, top }) => (
          <button
            key={bucket.year}
            aria-current={bucket.year === current.year ? 'date' : undefined}
            aria-label={`Jump to ${bucket.year}, ${bucket.count} photos`}
            style={{ top }}
            onClick={() => onYear(bucket.year)}
          >
            {bucket.year}
          </button>
        ))}
      </div>
      <input
        className="date-scrubber"
        type="range"
        aria-label="Jump to year or month"
        aria-valuetext={periodLabel(periods[value]?.period)}
        min="0"
        max={Math.max(1, periods.length - 1)}
        value={value}
        disabled={periods.length <= 1}
        onFocus={() => {
          setDraft(activeIndex);
          setFocused(true);
        }}
        onBlur={() => setFocused(false)}
        onPointerDown={() => {
          setDraft(activeIndex);
          setDragging(true);
        }}
        onChange={(event) => {
          setDraft(Number(event.target.value));
          setHovered(null);
        }}
        onPointerUp={(event) => {
          commit(Number(event.currentTarget.value));
          setDragging(false);
        }}
        onPointerCancel={() => setDragging(false)}
        onKeyUp={(event) => {
          if (
            [
              'ArrowUp',
              'ArrowDown',
              'ArrowLeft',
              'ArrowRight',
              'Home',
              'End',
              'PageUp',
              'PageDown',
            ].includes(event.key)
          )
            commit(Number(event.currentTarget.value));
        }}
        onPointerMove={(event) => {
          if (dragging) return;
          const bounds = event.currentTarget.getBoundingClientRect();
          setHovered(
            Math.max(
              0,
              Math.min(
                periods.length - 1,
                Math.round(
                  ((event.clientY - bounds.top) / Math.max(1, bounds.height)) *
                    (periods.length - 1),
                ),
              ),
            ),
          );
        }}
        onPointerLeave={() => setHovered(null)}
      />
      <div className="date-scrubber-track" aria-hidden="true">
        <i style={{ top: `${(value / Math.max(1, periods.length - 1)) * 100}%` }} />
      </div>
      <output
        className="timeline-current"
        aria-live="off"
        aria-label="Currently visible date"
        style={{
          top: `clamp(0px, ${(labelIndex / Math.max(1, periods.length - 1)) * 100}%, calc(100% - 24px))`,
        }}
      >
        {hovered === null && !dragging && !focused ? current.label : label}
      </output>
    </nav>
  );
}
