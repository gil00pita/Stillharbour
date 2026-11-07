import { Box, Heading, Text, chakra } from '@chakra-ui/react';
import type {
  InspectionFinding,
  InspectionVolume,
  LibraryRoot,
  StorageSummary,
} from '../../../bindings';
import { displayBytes } from '../../../inspection/format';
import { Button } from '@/components/Button';
import {
  categories,
  categoryColors,
  categoryOf,
  driveSegments,
  percentage,
  proportionalWidth,
  totalBytes,
} from '../model';

export function StorageOverview({
  summary,
  volumes,
  roots,
  files,
  complete,
  selected,
  onSelect,
}: {
  summary: StorageSummary;
  volumes: InspectionVolume[];
  roots: LibraryRoot[];
  files: InspectionFinding[];
  complete: boolean;
  selected: string;
  onSelect: (category: string) => void;
}) {
  const loadedBytes = totalBytes(files);
  return (
    <chakra.section className="storage-overview" aria-label="Storage overview">
      <Box className="storage-overview-heading">
        <Heading as="h2" unstyled>
          Selected libraries
        </Heading>
        <strong>{displayBytes(summary.total.logicalBytes)} in scanned files</strong>
      </Box>
      {volumes.length === 0 && <Text>Drive capacity unavailable · Free space unknown</Text>}
      {volumes.map((volume) => {
        const segments = driveSegments(volume.totalBytes, volume.availableBytes);
        const name = volume.rootIds
          .map((id) => roots.find((root) => root.id === id)?.label ?? 'Previously registered root')
          .join(', ');
        return (
          <chakra.section key={volume.id} aria-label={`Drive usage for ${name}`}>
            <Box className="storage-overview-heading">
              <b>{name}</b>
              <Text as="span">
                {segments
                  ? `${displayBytes(segments.used)} of ${displayBytes(segments.capacity)} used · ${displayBytes(segments.free)} available`
                  : `Capacity: ${displayBytes(volume.totalBytes)} · Free space ${volume.availableBytes === null ? 'unknown' : displayBytes(volume.availableBytes)}`}
              </Text>
            </Box>
            {segments && (
              <Box
                className="storage-bar"
                aria-label={`Drive storage: ${displayBytes(segments.free)} available`}
              >
                {(
                  [
                    ['Used space', segments.used, 'var(--secondary)'],
                    ['Free space', segments.free, 'var(--border)'],
                  ] as const
                ).map(
                  ([label, bytes, color]) =>
                    byteValuePositive(bytes) && (
                      <Text
                        as="span"
                        key={label}
                        tabIndex={0}
                        style={{
                          width: proportionalWidth(bytes, segments.capacity),
                          background: color,
                        }}
                        title={`${label} · ${displayBytes(bytes)} · ${percentage(bytes, segments.capacity)}`}
                        aria-label={`${label}: ${displayBytes(bytes)}, ${percentage(bytes, segments.capacity)}`}
                      />
                    ),
                )}
              </Box>
            )}
            <small>
              Observed {volume.observedAt} · {volume.coverage}. Used space includes files outside
              this inspection.
            </small>
          </chakra.section>
        );
      })}
      <Box className="storage-overview-heading">
        <b>File types · logical size</b>
        <Text as="span">
          {complete ? 'Loaded inspection' : 'Loaded file observations only'} ·{' '}
          {displayBytes(loadedBytes)}
        </Text>
      </Box>
      <Box className="storage-bar" aria-label="Scanned file-type breakdown">
        {categories.map((category) => {
          const entries = files.filter((file) => categoryOf(file) === category);
          const bytes = totalBytes(entries);
          return (
            byteValuePositive(bytes) && (
              <Button
                key={category}
                aria-label={`${category}: ${displayBytes(bytes)}, ${percentage(bytes, loadedBytes)}, ${entries.length} files`}
                aria-pressed={selected === category}
                style={{
                  width: proportionalWidth(bytes, loadedBytes),
                  background: categoryColors[category],
                }}
                title={`${category} · ${displayBytes(bytes)} · ${percentage(bytes, loadedBytes)} of loaded logical size`}
                onClick={() => onSelect(selected === category ? '' : category)}
              />
            )
          );
        })}
      </Box>
      <Box className="storage-legend" aria-label="File-type legend">
        {categories.map((category) => {
          const entries = files.filter((file) => categoryOf(file) === category);
          return (
            entries.length > 0 && (
              <Button
                key={category}
                aria-pressed={selected === category}
                onClick={() => onSelect(selected === category ? '' : category)}
              >
                <i style={{ background: categoryColors[category] }} />
                {category} · {displayBytes(totalBytes(entries))}
              </Button>
            )
          );
        })}
      </Box>
      <details>
        <summary>How storage is counted · Read only</summary>
        <Text>
          Drive usage is measured by the filesystem. File-type totals describe logical file sizes
          and may count hard-linked paths more than once; they cannot be subtracted from physical
          drive usage. Unknown or unscanned used space remains included in Used space. Filters,
          navigation and simulations never change these totals.
        </Text>
        <Text>
          Hard links counted once: {displayBytes(summary.total.uniqueObjectLogicalBytes)} · Reported
          allocation: {displayBytes(summary.total.reportedAllocatedBytes)}. Recoverable disk space
          is unknown. Same-volume quarantine frees {displayBytes(summary.quarantineFreedBytes)}.
        </Text>
        {summary.accountingIncomplete && (
          <Text>Object accounting reached its bound; unknown metrics remain unmeasured.</Text>
        )}
      </details>
    </chakra.section>
  );
}
function byteValuePositive(value: string) {
  return value !== '0';
}
