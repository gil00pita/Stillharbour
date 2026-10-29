import { useEffect, useEffectEvent, useMemo, useRef, useState } from 'react';
import type { DateBucket } from '../bindings';
import { timelineDate } from './model';
import { periodLabel, timelineYears } from './timeline';

export function DateTimeline({
  buckets,
  date,
  scrolling,
  onYear,
  onScroll,
  progress = 0,
  onPosition,
}: {
  buckets: DateBucket[];
  date: string;
  scrolling: boolean;
  onYear: (period: string) => void;
  onScroll?: (delta: number, mode: number) => void;
  progress?: number;
  onPosition?: (period: string, progress: number) => boolean;
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
  const maximum = Math.max(1, periods.length);
  const value =
    dragging || editing
      ? Math.min(draft, maximum)
      : activeIndex + Math.max(0, Math.min(0.9999, progress));
  const labelIndex = hovered ?? value;
  const period = periods[Math.min(periods.length - 1, Math.floor(labelIndex))]?.period;
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
    const index = positionAt(event.clientY, bounds);
    draftValue.current = index;
    setDraft(index);
    scrollLoaded(index);
  });
  const finishDrag = useEffectEvent((event: PointerEvent) => {
    if (event.pointerId !== dragPointer.current) return;
    if (event.type === 'pointerup') moveDrag(event);
    dragPointer.current = null;
    setDragging(false);
    setEditing(false);
    if (event.type === 'pointerup') commit(draftValue.current);
  });
  useEffect(() => {
    const move = (event: PointerEvent) => moveDrag(event);
    const finish = (event: PointerEvent) => finishDrag(event);
    // Subscribe before a gesture starts, so a quick release cannot race an
    // effect installation. The pointer ID gates all unrelated document input.
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
  }, []);
  const visibleYears = timelineYears(buckets, periods, current.year, height);
  function positionAt(y: number, bounds: { top: number; height: number }) {
    if (!Number.isFinite(y) || bounds.height <= 0) return draftValue.current;
    return Math.max(0, Math.min(maximum, ((y - bounds.top) / bounds.height) * maximum));
  }
  function targetAt(position: number) {
    const index = Math.max(0, Math.min(periods.length - 1, Math.floor(position)));
    return { period: periods[index]?.period, progress: Math.min(1, Math.max(0, position - index)) };
  }
  function scrollLoaded(position: number) {
    const target = targetAt(position);
    return target.period === currentPeriod ? onPosition?.(target.period, target.progress) : false;
  }
  function commit(position: number) {
    const target = targetAt(position);
    if (target.period && !scrollLoaded(position)) onYear(target.period);
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
        title="Drag to scroll photos or jump to a month."
        aria-label="Jump to year or month"
        aria-valuetext={periodLabel(
          periods[Math.min(periods.length - 1, Math.floor(value))]?.period,
        )}
        min="0"
        max={maximum}
        step="any"
        value={value}
        disabled={periods.length === 0}
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
          const index = positionAt(event.clientY, bounds);
          draftValue.current = index;
          setDraft(index);
          setHovered(null);
          dragPointer.current = event.pointerId;
          if (event.isTrusted) event.currentTarget.setPointerCapture(event.pointerId);
          setDragging(true);
          scrollLoaded(index);
        }}
        onChange={(event) => {
          const next = Number(event.target.value);
          draftValue.current = next;
          setDraft(next);
          setEditing(true);
          setHovered(null);
        }}
        onKeyDown={(event) => {
          const direction = {
            ArrowDown: 0.05,
            ArrowRight: 0.05,
            ArrowUp: -0.05,
            ArrowLeft: -0.05,
            PageDown: 0.5,
            PageUp: -0.5,
          }[event.key];
          if (direction === undefined && !['Home', 'End'].includes(event.key)) return;
          event.preventDefault();
          const next =
            event.key === 'Home'
              ? 0
              : event.key === 'End'
                ? maximum
                : Math.max(0, Math.min(maximum, value + (direction ?? 0)));
          draftValue.current = next;
          setDraft(next);
          setEditing(true);
          scrollLoaded(next);
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
          setHovered(positionAt(event.clientY, bounds));
        }}
        onPointerLeave={() => setHovered(null)}
      />
      <div className="date-scrubber-track" aria-hidden="true">
        <i style={{ top: `${(value / maximum) * 100}%` }} />
      </div>
      <output
        className="timeline-current"
        aria-live="off"
        aria-label="Currently visible date"
        style={{
          top: `clamp(0px, ${(labelIndex / maximum) * 100}%, calc(100% - 24px))`,
        }}
      >
        {hovered === null && !dragging && !focused ? current.label : label}
      </output>
    </nav>
  );
}
