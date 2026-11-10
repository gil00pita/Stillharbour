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
    <Box
      className="date-heading"
      css={{
        position: 'sticky',
        zIndex: '3',
        background: '{colors.bg}',
        top: '0',
        height: '36px',
        display: 'flex',
        alignItems: 'center',
        gap: '7px',
        minWidth: '0',
        overflow: 'hidden',
        '& h2': {
          fontSize: '13px',
          fontWeight: '600',
          letterSpacing: '0',
          overflow: 'hidden',
          whiteSpace: 'nowrap',
          textOverflow: 'ellipsis',
          margin: '0',
        },
        '& small': { fontSize: '11px', color: '{colors.fg.muted}', whiteSpace: 'nowrap' },
        '& input': { opacity: '0', margin: '0' },
        '&:hover input': { opacity: '1' },
        '&:focus-within input': { opacity: '1' },
        '& input:checked': { opacity: '1' },
      }}
      data-day={heading.date}
      style={{ width: heading.width }}
    >
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
