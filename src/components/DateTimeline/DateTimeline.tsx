import { useEffect, useEffectEvent, useId, useMemo, useRef, useState } from 'react';
import type { DateBucket } from '../../bindings';
import {
  periodLabel,
  timelineKeyPosition,
  timelinePositionAt,
  timelineScrollState,
  timelineTarget,
  timelineTicks,
  timelineYears,
} from '../../library/timeline';

export function DateTimeline({
  buckets,
  date,
  scrolling,
  onYear,
  onScroll,
  progress = 0,
  onPosition,
  index,
  onIndex,
  pageKey = '',
}: {
  buckets: DateBucket[];
  date: string;
  scrolling: boolean;
  onYear: (period: string) => void;
  onScroll?: (delta: number, mode: number) => void;
  progress?: number;
  onPosition?: (period: string, progress: number) => boolean;
  index?: number;
  onIndex?: (index: number) => boolean;
  pageKey?: string;
}) {
  const rail = useRef<HTMLElement>(null);
  const slider = useRef<HTMLInputElement>(null);
  const instructions = useId();
  const [height, setHeight] = useState(400);
  const [draft, setDraft] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [editing, setEditing] = useState(false);
  const [focused, setFocused] = useState(false);
  const [hovered, setHovered] = useState<number | null>(null);
  const [pending, setPending] = useState<{
    index: number;
    pageKey: string;
    observedIndex?: number;
  } | null>(null);
  const dragPointer = useRef<number | null>(null);
  const draftValue = useRef(0);
  const lastEmitted = useRef<number | null>(null);
  const periods = useMemo(
    () =>
      buckets
        .flatMap((bucket) =>
          bucket.months.length ? bucket.months : [{ period: bucket.year, count: bucket.count }],
        )
        .filter((item) => item.count > 0),
    [buckets],
  );
  const {
    current,
    period: currentPeriod,
    maximum,
    position: inferred,
  } = timelineScrollState(periods, date, progress);
  const position = Math.max(0, Math.min(maximum, index ?? inferred));
  if (
    pending &&
    (pending.pageKey !== pageKey ||
      (pending.observedIndex !== undefined && pending.observedIndex !== position))
  ) {
    setPending(null);
  }
  const waiting =
    pending?.pageKey === pageKey &&
    (pending.observedIndex === undefined || pending.observedIndex === position);
  const value = dragging || editing ? Math.min(draft, maximum) : waiting ? pending.index : position;
  const labelIndex = hovered ?? value;
  const label = periodLabel(timelineTarget(periods, labelIndex).period);
  const denominator = Math.max(1, maximum);
  useEffect(() => {
    const element = rail.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) =>
      setHeight(entries[0]?.contentRect.height ?? 400),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  function stopGesture() {
    dragPointer.current = null;
    setDragging(false);
    setEditing(false);
    setHovered(null);
  }
  function navigate(next: number) {
    const target = timelineTarget(periods, next);
    if (lastEmitted.current === target.index && dragPointer.current !== null) return;
    lastEmitted.current = target.index;
    if (onIndex) {
      const loaded = onIndex(target.index);
      // A loaded photo can share the current gallery row. Preserve its requested
      // ordinal across key releases until the visible row or page changes.
      setPending({ index: target.index, pageKey, ...(loaded ? { observedIndex: position } : {}) });
    } else if (
      target.period &&
      !(target.period === currentPeriod && onPosition?.(target.period, target.progress))
    ) {
      onYear(target.period);
    }
  }
  const scrollEnabled = Boolean(onScroll);
  const wheelInput = useEffectEvent((event: WheelEvent) => {
    if (!onScroll || event.ctrlKey || event.deltaY === 0) return;
    setPending(null);
    setHovered(null);
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
    const next = timelinePositionAt(event.clientY, bounds, maximum, draftValue.current);
    draftValue.current = next;
    setDraft(next);
    navigate(next);
  });
  const finishDrag = useEffectEvent((event: PointerEvent) => {
    if (event.pointerId !== dragPointer.current) return;
    if (event.type === 'pointerup') moveDrag(event);
    stopGesture();
  });
  useEffect(() => {
    const move = (event: PointerEvent) => moveDrag(event);
    const finish = (event: PointerEvent) => finishDrag(event);
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
  return (
    <nav
      ref={rail}
      className={`date-rail ${scrolling || dragging || focused ? 'is-scrolling' : ''}`}
      aria-label="Photo timeline"
      data-dragging={dragging}
      onPointerLeave={() => setHovered(null)}
    >
      <span id={instructions} className="screen-reader-status">
        Drag to navigate through photos. Arrow keys move one photo; Home and End go to the newest
        and oldest photos. Escape stops dragging.
      </span>
      <div className="timeline-year-labels">
        {timelineYears(buckets, periods, current.year, height).map(({ bucket, top }) => (
          <button
            key={bucket.year}
            aria-current={bucket.year === current.year ? 'date' : undefined}
            aria-label={`Jump to ${bucket.year}, ${bucket.count} photos`}
            style={{ top }}
            onClick={() => {
              setPending(null);
              onYear(bucket.year);
            }}
          >
            {bucket.year}
          </button>
        ))}
      </div>
      <input
        ref={slider}
        className="date-scrubber"
        type="range"
        aria-label="Jump to year or month"
        aria-describedby={instructions}
        aria-orientation="vertical"
        aria-valuetext={periodLabel(timelineTarget(periods, value).period)}
        min="0"
        max={maximum}
        step="1"
        value={value}
        disabled={maximum === 0}
        onFocus={() => setFocused(true)}
        onBlur={() => {
          setFocused(false);
          stopGesture();
          setPending((current) => (current?.observedIndex === undefined ? current : null));
        }}
        onPointerDown={(event) => {
          if (event.button > 0 || dragPointer.current !== null) return;
          event.preventDefault();
          event.currentTarget.focus({ preventScroll: true });
          const next = timelinePositionAt(
            event.clientY,
            event.currentTarget.getBoundingClientRect(),
            maximum,
            value,
          );
          draftValue.current = next;
          lastEmitted.current = null;
          dragPointer.current = event.pointerId;
          setDraft(next);
          setHovered(null);
          setDragging(true);
          if (event.isTrusted) event.currentTarget.setPointerCapture(event.pointerId);
          navigate(next);
        }}
        onChange={(event) => {
          const next = Number(event.target.value);
          draftValue.current = next;
          setDraft(next);
          navigate(next);
        }}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault();
            stopGesture();
            return;
          }
          const next = timelineKeyPosition(event.key, value, maximum);
          if (next === null) return;
          event.preventDefault();
          lastEmitted.current = null;
          draftValue.current = next;
          setDraft(next);
          setEditing(true);
          setHovered(null);
          navigate(next);
        }}
        onKeyUp={(event) => {
          if (timelineKeyPosition(event.key, value, maximum) !== null) setEditing(false);
        }}
        onPointerMove={(event) => {
          if (!dragging)
            setHovered(
              timelinePositionAt(
                event.clientY,
                event.currentTarget.getBoundingClientRect(),
                maximum,
                value,
              ),
            );
        }}
        onPointerLeave={() => setHovered(null)}
      />
      <div className="date-scrubber-track">
        {timelineTicks(periods, height).map((tick) => (
          <span
            key={tick.period}
            className="timeline-tick"
            style={{ top: `${tick.position * 100}%` }}
          />
        ))}
        <i style={{ top: `${(value / denominator) * 100}%` }} />
        <span
          className="timeline-preview-marker"
          aria-hidden="true"
          style={{ top: `${(labelIndex / denominator) * 100}%` }}
        />
        <output
          className="timeline-current"
          aria-live="off"
          aria-label={
            hovered !== null || dragging || editing
              ? 'Timeline preview date'
              : 'Currently visible date'
          }
          style={{ top: `${(labelIndex / denominator) * 100}%` }}
        >
          {label}
        </output>
      </div>
    </nav>
  );
}
