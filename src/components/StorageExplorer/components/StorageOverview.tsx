import type {
  InspectionFinding,
  InspectionVolume,
  LibraryRoot,
  StorageSummary,
} from '../../../bindings';
import { displayBytes } from '../../../inspection/format';
import { Button } from '../../ui/controls';
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
    <section className="storage-overview" aria-label="Storage overview">
      <div className="storage-overview-heading">
        <h2>Selected libraries</h2>
        <strong>{displayBytes(summary.total.logicalBytes)} in scanned files</strong>
      </div>
      {volumes.length === 0 && <p>Drive capacity unavailable · Free space unknown</p>}
      {volumes.map((volume) => {
        const segments = driveSegments(volume.totalBytes, volume.availableBytes);
        const name = volume.rootIds
          .map((id) => roots.find((root) => root.id === id)?.label ?? 'Previously registered root')
          .join(', ');
        return (
          <section key={volume.id} aria-label={`Drive usage for ${name}`}>
            <div className="storage-overview-heading">
              <b>{name}</b>
              <span>
                {segments
                  ? `${displayBytes(segments.used)} of ${displayBytes(segments.capacity)} used · ${displayBytes(segments.free)} available`
                  : `Capacity: ${displayBytes(volume.totalBytes)} · Free space ${volume.availableBytes === null ? 'unknown' : displayBytes(volume.availableBytes)}`}
              </span>
            </div>
            {segments && (
              <div
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
                      <span
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
              </div>
            )}
            <small>
              Observed {volume.observedAt} · {volume.coverage}. Used space includes files outside
              this inspection.
            </small>
          </section>
        );
      })}
      <div className="storage-overview-heading">
        <b>File types · logical size</b>
        <span>
          {complete ? 'Loaded inspection' : 'Loaded file observations only'} ·{' '}
          {displayBytes(loadedBytes)}
        </span>
      </div>
      <div className="storage-bar" aria-label="Scanned file-type breakdown">
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
      </div>
      <div className="storage-legend" aria-label="File-type legend">
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
      </div>
      <details>
        <summary>How storage is counted · Read only</summary>
        <p>
          Drive usage is measured by the filesystem. File-type totals describe logical file sizes
          and may count hard-linked paths more than once; they cannot be subtracted from physical
          drive usage. Unknown or unscanned used space remains included in Used space. Filters,
          navigation and simulations never change these totals.
        </p>
        <p>
          Hard links counted once: {displayBytes(summary.total.uniqueObjectLogicalBytes)} · Reported
          allocation: {displayBytes(summary.total.reportedAllocatedBytes)}. Recoverable disk space
          is unknown. Same-volume quarantine frees {displayBytes(summary.quarantineFreedBytes)}.
        </p>
        {summary.accountingIncomplete && (
          <p>Object accounting reached its bound; unknown metrics remain unmeasured.</p>
        )}
      </details>
    </section>
  );
}
function byteValuePositive(value: string) {
  return value !== '0';
}
