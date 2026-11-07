/*! Timeline geometry adapted from relyfe/timeline-vertical-navigation.
MIT License

Copyright (c) 2018

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/
import { Box, Text, chakra, Button } from '@chakra-ui/react';
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
    <chakra.nav
      ref={rail}
      className={`date-rail ${scrolling || dragging || focused ? 'is-scrolling' : ''}`}
      css={{
        '--timeline-line': '{colors.brand.solid}',
        position: 'relative',
        width: '32px',
        minHeight: '0',
        margin: '0',
        isolation: 'isolate',
        userSelect: 'none',
        opacity: '1',
        '&:hover': { opacity: '1', transition: 'none' },
        '&:focus-within': { opacity: '1', transition: 'none' },
        '&.is-scrolling': { opacity: '1', transition: 'none' },
        '& button': {
          position: 'absolute',
          right: '0',
          border: '0 !important',
          background: '{colors.bg.subtle} !important',
          padding: '0 !important',
          width: '32px',
          minHeight: '20px !important',
          borderRadius: '10px',
          color: '{colors.fg} !important',
          fontSize: '11px !important',
          lineHeight: '20px !important',
          letterSpacing: '0',
          fontWeight: '500',
          fontVariantNumeric: 'tabular-nums',
          whiteSpace: 'nowrap',
        },
        '@media (prefers-reduced-motion: reduce)': { transition: 'none' },
        '@media (max-width: 760px)': { width: '32px' },
      }}
      aria-label="Photo timeline"
      data-dragging={dragging}
      onPointerLeave={() => setHovered(null)}
    >
      <Text
        as="span"
        id={instructions}
        className="screen-reader-status"
        css={{
          position: 'absolute',
          width: '1px',
          height: '1px',
          overflow: 'hidden',
          clipPath: 'inset(50%)',
        }}
      >
        Drag to navigate through photos. Arrow keys move one photo; Home and End go to the newest
        and oldest photos. Escape stops dragging.
      </Text>
      <Box
        className="timeline-year-labels"
        css={{ position: 'absolute', inset: '24px 0', zIndex: '1' }}
      >
        {timelineYears(buckets, periods, current.year, height).map(({ bucket, top }) => (
          <Button
            unstyled
            type="submit"
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
          </Button>
        ))}
      </Box>
      <chakra.input
        ref={slider}
        className="date-scrubber"
        css={{
          position: 'absolute',
          top: '24px',
          right: '0',
          width: '100% !important',
          height: 'calc(100% - 48px)',
          minHeight: '0',
          padding: '0 !important',
          margin: '0',
          border: '0 !important',
          background: 'transparent !important',
          writingMode: 'vertical-lr',
          direction: 'ltr',
          touchAction: 'none',
          cursor: 'row-resize',
          opacity: '0',
          zIndex: '2',
          '&:focus-visible + .date-scrubber-track i': {
            outline: '2px solid var(--timeline-line)',
            outlineOffset: '3px',
          },
        }}
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
      <Box
        className="date-scrubber-track"
        css={{
          position: 'absolute',
          top: '24px',
          bottom: '24px',
          right: '4px',
          width: '0',
          pointerEvents: 'none',
          '& i': {
            position: 'absolute',
            right: '0',
            width: '24px',
            height: '1px',
            background: 'var(--timeline-line)',
          },
        }}
      >
        {timelineTicks(periods, height).map((tick) => (
          <Text
            as="span"
            key={tick.period}
            className="timeline-tick"
            css={{
              position: 'absolute',
              width: '4px',
              height: '4px',
              borderRadius: '50%',
              background: '{colors.fg.secondary}',
              transform: 'translateX(-14px)',
            }}
            style={{ top: `${tick.position * 100}%` }}
          />
        ))}
        <i style={{ top: `${(value / denominator) * 100}%` }} />
        <Text
          as="span"
          className="timeline-preview-marker"
          css={{
            position: 'absolute',
            right: '0',
            width: '24px',
            height: '2px',
            zIndex: '3',
            background: 'var(--timeline-line)',
            boxShadow: '0 1px 2px rgb(60 64 67 / 30%),\n    0 2px 6px 2px rgb(60 64 67 / 15%)',
            pointerEvents: 'none',
          }}
          aria-hidden="true"
          style={{ top: `${(labelIndex / denominator) * 100}%` }}
        />
        <chakra.output
          className="timeline-current"
          opacity={hovered !== null || dragging || focused || scrolling ? 1 : 0}
          css={{
            maxWidth: '92px',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            position: 'absolute',
            right: '32px',
            zIndex: '2',
            height: '24px',
            padding: '2px 10px',
            border: '0',
            borderRadius: '2px',
            background: 'bg.subtle',
            color: '{colors.fg}',
            fontSize: '12px',
            fontWeight: '500',
            lineHeight: '20px',
            letterSpacing: '0',
            fontVariantNumeric: 'tabular-nums',
            whiteSpace: 'nowrap',
            pointerEvents: 'none',
            transform: 'translateY(-100%)',
          }}
          aria-live="off"
          aria-label={
            hovered !== null || dragging || editing
              ? 'Timeline preview date'
              : 'Currently visible date'
          }
          style={{ top: `${(labelIndex / denominator) * 100}%` }}
        >
          {label}
        </chakra.output>
      </Box>
    </chakra.nav>
  );
}
