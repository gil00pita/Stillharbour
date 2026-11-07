import { Box, Text } from '@chakra-ui/react';
import type { LibraryOrganizationSummary } from '../../../bindings';

export function StructureSummary({
  summary,
  simulated = false,
}: {
  summary: LibraryOrganizationSummary;
  simulated?: boolean;
}) {
  return (
    <Box
      as="section"
      className="structure-review-summary"
      aria-label={simulated ? 'Simulated changes' : 'Proposed changes'}
    >
      {[
        [summary.moves, simulated ? 'Simulated moves' : 'Photos & videos to move'],
        [summary.folders, 'Destination folders'],
        [summary.renames, simulated ? 'Simulated renames' : 'Files to rename'],
        [summary.missingDates, 'Missing capture date'],
      ].map(([count, label]) => (
        <Box key={label}>
          <Text as="strong">{typeof count === 'number' ? count.toLocaleString() : count}</Text>
          <Text as="span">{label}</Text>
        </Box>
      ))}
    </Box>
  );
}
