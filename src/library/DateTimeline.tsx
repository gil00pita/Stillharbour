import { useEffect, useEffectEvent, useMemo, useRef, useState } from 'react';
import type { DateBucket } from '../bindings';
import { timelineDate } from './model';
import { periodLabel, timelineIndexAt, timelineYears } from './timeline';

export function DateTimeline({
  buckets,
  date,
  scrolling,
  onYear,
  onScroll,
}: {
  buckets: DateBucket[];
  date: string;
  scrolling: boolean;
  onYear: (period: string) => void;
  onScroll?: (delta: number, mode: number) => void;
}) {
  const rail = useRef<HTMLElement>(null);
  const [height, setHeight] = useState(400);
  const [draft, setDraft] = useState(0);
  const [dragging, setDragging] = useState(false);
  const slider = useRef<HTMLInputElement>(null);
  const dragPointer = useRef<number | null>(null);
  const draftValue = useRef(0);
  const [editing, setEditing] = useState(false);
  const [focused, setFocused] = useState(false);
  const [hovered, setHovered] = useState<number | null>(null);
  const periods = useMemo(
    () =>
      buckets.flatMap((bucket) =>
        bucket.months.length ? bucket.months : [{ period: bucket.year, count: bucket.count }],
      ),
    [buckets],
  );
  const current = timelineDate(date);
  const currentPeriod = current.year === 'Undated' ? 'Undated' : date.slice(0, 7);
  const activeIndex = Math.max(
    0,
    periods.findIndex((period) => period.period === currentPeriod),
  );
  const value =
    dragging || editing ? Math.min(draft, Math.max(0, periods.length - 1)) : activeIndex;
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
  const scrollEnabled = Boolean(onScroll);
  const wheelInput = useEffectEvent((event: WheelEvent) => {
    if (!onScroll || event.ctrlKey || event.deltaY === 0) return;
    event.preventDefault();
    onScroll(event.deltaY, event.deltaMode);
  });
  useEffect(() => {
    const element = rail.current;
    if (!element || !scrollEnabled) return;
    const wheel = (event: WheelEvent) => wheelInput(event);
    element.addEventListener('wheel', wheel, { passive: false });
    return () => element.removeEventListener('wheel', wheel);
  }, [scrollEnabled]);
  const moveDrag = useEffectEvent((event: PointerEvent) => {
    if (event.pointerId !== dragPointer.current) return;
    const bounds = slider.current?.getBoundingClientRect();
    if (!bounds || !Number.isFinite(event.clientY) || bounds.height <= 0) return;
    const index = timelineIndexAt(event.clientY, bounds, periods.length, draftValue.current);
    draftValue.current = index;
    setDraft(index);
  });
  const finishDrag = useEffectEvent((event: PointerEvent) => {
    if (event.pointerId !== dragPointer.current) return;
    const selected = periods[draftValue.current];
    dragPointer.current = null;
    setDragging(false);
    setEditing(false);
    if (event.type === 'pointerup' && selected) onYear(selected.period);
  });
  useEffect(() => {
    if (!dragging) return;
    const move = (event: PointerEvent) => moveDrag(event);
    const finish = (event: PointerEvent) => finishDrag(event);
    // Keep one subscription for the gesture, reading current periods/callbacks.
    // Native pointer capture also handles release outside the window.
    document.addEventListener('pointermove', move);
    document.addEventListener('pointerup', finish);
    document.addEventListener('pointercancel', finish);
    document.addEventListener('lostpointercapture', finish);
    return () => {
      document.removeEventListener('pointermove', move);
      document.removeEventListener('pointerup', finish);
      document.removeEventListener('pointercancel', finish);
      document.removeEventListener('lostpointercapture', finish);
    };
  }, [dragging]);
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
        ref={slider}
        className="date-scrubber"
        type="range"
        title="Jump to a month. Use the gallery scrollbar for precise scrolling."
        aria-label="Jump to year or month"
        aria-valuetext={periodLabel(periods[value]?.period)}
        min="0"
        max={Math.max(1, periods.length - 1)}
        value={value}
        disabled={periods.length <= 1}
        onFocus={() => setFocused(true)}
        onBlur={() => {
          setFocused(false);
          setEditing(false);
        }}
        onPointerDown={(event) => {
          if (event.button > 0 || dragPointer.current !== null) return;
          event.preventDefault();
          event.currentTarget.focus({ preventScroll: true });
          const bounds = event.currentTarget.getBoundingClientRect();
          const index = timelineIndexAt(event.clientY, bounds, periods.length, activeIndex);
          draftValue.current = index;
          setDraft(index);
          setHovered(null);
          dragPointer.current = event.pointerId;
          if (event.isTrusted) event.currentTarget.setPointerCapture(event.pointerId);
          setDragging(true);
        }}
        onChange={(event) => {
          const next = Number(event.target.value);
          draftValue.current = next;
          setDraft(next);
          setEditing(true);
          setHovered(null);
        }}
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
          ) {
            commit(Number(event.currentTarget.value));
            setEditing(false);
          }
        }}
        onPointerMove={(event) => {
          if (dragging) return;
          const bounds = event.currentTarget.getBoundingClientRect();
          setHovered(timelineIndexAt(event.clientY, bounds, periods.length));
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
