import type { DuplicateGroup, InspectionFinding } from '../../bindings';

export type KeeperRule = 'resolution' | 'raw' | 'largest' | 'newest' | 'oldest' | 'albums';
export type CleanupPolicy = { rule: KeeperRule; threshold: number; protections: string[] };
export const initialCleanupPolicy: CleanupPolicy = {
  rule: 'resolution',
  threshold: 97,
  protections: ['Favourites', 'RAW files', 'Video masters', 'Shared albums', 'Edited derivatives'],
};
export const keeperRules: { value: KeeperRule; label: string; unavailable?: string }[] = [
  { value: 'resolution', label: 'Highest resolution' },
  { value: 'raw', label: 'Original / RAW file' },
  { value: 'largest', label: 'Largest file' },
  {
    value: 'newest',
    label: 'Newest edit',
    unavailable:
      'Edit history is not available to duplicate inspection. Filesystem modification is not edit history.',
  },
  {
    value: 'oldest',
    label: 'Oldest capture',
    unavailable:
      'Capture-time provenance is not available to keeper rules. Filesystem birth time is not capture time.',
  },
  {
    value: 'albums',
    label: 'Appears in more albums',
    unavailable: 'Album membership is not available to keeper rules.',
  },
];
export const evidenceLabels: Record<DuplicateGroup['evidence'], string> = {
  exact_bytes: 'Exact byte match',
  normalized_pixels: 'Matching normalized pixels',
  property_candidate: 'Similar file properties',
  hard_link_aliases: 'Linked paths · one object',
};
const raw = /\.(?:raw|dng|cr2|cr3|nef|nrw|arw|orf|raf|rw2|pef|srw)$/i;
export function suggestKeepers(rows: InspectionFinding[], rule: KeeperRule): Set<string> {
  if (!rows.length || keeperRules.find((item) => item.value === rule)?.unavailable)
    return new Set();
  // Unknown evidence and ties keep multiple. Suggestions never authorise removal.
  if (rule === 'resolution' && rows.some((row) => !row.width || !row.height))
    return new Set(rows.map((row) => row.id));
  const value = (row: InspectionFinding): bigint => {
    if (rule === 'largest') return BigInt(row.bytes);
    if (rule === 'raw') return raw.test(row.name) ? 1n : 0n;
    return BigInt(row.width ?? 0) * BigInt(row.height ?? 0);
  };
  const maximum = rows.reduce((best, row) => (value(row) > best ? value(row) : best), 0n);
  return new Set(rows.filter((row) => value(row) === maximum).map((row) => row.id));
}
export function duplicateBytes(group: DuplicateGroup, rows: InspectionFinding[]): string | null {
  if (
    group.evidence !== 'exact_bytes' ||
    group.membersTruncated ||
    rows.length !== group.memberCount ||
    rows.some((row) => row.links !== '1' || row.inventory !== 'present' || !row.objectId) ||
    new Set(rows.map((row) => row.objectId)).size !== rows.length ||
    new Set(rows.map((row) => row.bytes)).size !== 1
  )
    return null;
  // Logical duplicate bytes only. Quarantine does not free disk blocks.
  const largest = rows.reduce(
    (maximum, row) => (BigInt(row.bytes) > maximum ? BigInt(row.bytes) : maximum),
    0n,
  );
  return (rows.reduce((total, row) => total + BigInt(row.bytes), 0n) - largest).toString();
}
