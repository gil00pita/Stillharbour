import { Box, Heading } from '@chakra-ui/react';
import { usePhotoSelection } from '@/components/PhotoAlbumActions';
import { dayLabel, type PhotoHeading } from '../../library/model';

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
  const selection = usePhotoSelection();
  return (
    <Box className="date-heading" data-day={heading.date} style={{ width: heading.width }}>
      {(!selection || selection.active) && (
        <input
          type="checkbox"
          aria-label={`Select ${heading.ids.length} shown photos from ${dayLabel(heading.date)}`}
          checked={heading.ids.length > 0 && heading.ids.every((id) => selected.has(id))}
          disabled={frozen}
          onChange={() => onSelect?.(heading.ids)}
        />
      )}
      <Heading as="h2" unstyled title={dayLabel(heading.date)} aria-label={dayLabel(heading.date)}>
        {dayLabel(heading.date, undefined, heading.width < 350)}
      </Heading>
      {heading.width >= 180 && <small>{heading.ids.length} shown</small>}
    </Box>
  );
}
