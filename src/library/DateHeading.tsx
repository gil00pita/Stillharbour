import { dayLabel, type PhotoHeading } from './model';

export function DateHeading({
  heading,
  selected,
  frozen,
  onSelect,
}: {
  heading: PhotoHeading;
  selected: ReadonlySet<string>;
  frozen: boolean;
  onSelect: ((ids: string[]) => void) | undefined;
}) {
  return (
    <div className="date-heading" data-day={heading.date} style={{ width: heading.width }}>
      <input
        type="checkbox"
        aria-label={`Select ${heading.ids.length} shown photos from ${dayLabel(heading.date)}`}
        checked={heading.ids.length > 0 && heading.ids.every((id) => selected.has(id))}
        disabled={frozen}
        onChange={() => onSelect?.(heading.ids)}
      />
      <h2 title={dayLabel(heading.date)} aria-label={dayLabel(heading.date)}>
        {dayLabel(heading.date, undefined, heading.width < 350)}
      </h2>
      {heading.width >= 180 && <small>{heading.ids.length} shown</small>}
    </div>
  );
}
