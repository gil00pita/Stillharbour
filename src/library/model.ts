import { diagnosticClock, diagnosticSince } from '../performance';
import { justifiedLayout } from './justifiedLayout';
import {
  DEFAULT_PHOTO_EXTENSIONS,
  type AssetQuery,
  type AssetSummary,
  type ScanFilter,
} from '../bindings';

export const defaultFilter: ScanFilter = {
  version: 1,
  includeGlobs: [],
  excludeGlobs: [],
  excludedFolders: [],
  extensions: [...DEFAULT_PHOTO_EXTENSIONS],
  includeHiddenFiles: false,
  includeHiddenFolders: false,
  minBytes: null,
  maxBytes: null,
  fromDate: null,
  throughDate: null,
};
export const defaultQuery: AssetQuery = {
  rootId: null,
  folder: '',
  name: '',
  fromDate: '',
  throughDate: '',
  minBytes: '',
  maxBytes: '',
  extension: '',
  favoritesOnly: false,
  tag: '',
  sort: 'capture',
  includeMissing: false,
};
export type Section = 'Photos' | 'Folders' | 'Tags';
export type ViewMode = 'gallery' | 'grid' | 'list';
export type PhotoHeading = { date: string; index: number; width: number; ids: string[] };
export type PhotoRow = {
  assets: AssetSummary[];
  height: number;
  imageHeight: number;
  widths: number[];
  headings: PhotoHeading[];
};
export type DateGrouping = 'capture' | 'added' | null;
export const PHOTO_GUTTER = 4;
export const DATE_HEADING_HEIGHT = 36;
export const THUMBNAIL_SIZES = { compact: 150, comfortable: 220, large: 300 } as const;
function ratio(asset: AssetSummary): number {
  return asset.width && asset.height && Number.isFinite(asset.width / asset.height)
    ? Math.max(0.05, Math.min(20, asset.width / asset.height))
    : 1.4;
}

// Geometry is bounded independently of untrusted metadata; the feed retains at most 720 assets.
export function photoRows(...args: Parameters<typeof computePhotoRows>): PhotoRow[] {
  const started = diagnosticClock();
  try {
    return computePhotoRows(...args);
  } finally {
    diagnosticSince('layout_ms', started);
  }
}

function computePhotoRows(
  assets: AssetSummary[],
  width: number,
  mode: ViewMode,
  target = 220,
  grouping: DateGrouping = 'capture',
): PhotoRow[] {
  const available = Number.isFinite(width) ? Math.max(160, width) : 160;
  if (mode === 'list')
    return assets.map((asset) => ({
      assets: [asset],
      height: 54,
      imageHeight: 0,
      headings: [],
      widths: [available],
    }));
  if (mode === 'grid') {
    const columns = Math.max(1, Math.floor((available + PHOTO_GUTTER) / Math.max(100, target)));
    const side = (available - (columns - 1) * PHOTO_GUTTER) / columns;
    return Array.from({ length: Math.ceil(assets.length / columns) }, (_, i) => ({
      assets: assets.slice(i * columns, (i + 1) * columns),
      height: side + 42,
      imageHeight: side,
      headings: [],
      widths: Array(columns).fill(side) as number[],
    }));
  }
  const rows: PhotoRow[] = [];
  const ideal = Number.isFinite(target) ? Math.max(100, Math.min(400, target)) : 220;
  const groups = new Map<string, string[]>();
  for (const asset of assets) {
    const day = assetDay(asset, grouping);
    const ids = groups.get(day) ?? [];
    ids.push(asset.id);
    groups.set(day, ids);
  }
  for (let start = 0; start < assets.length;) {
    const day = assetDay(assets[start]!, grouping);
    let end = grouping ? start + 1 : assets.length;
    while (grouping && end < assets.length && assetDay(assets[end]!, grouping) === day) end++;
    const section = assets.slice(start, end);
    for (const layout of justifiedLayout(section.map(ratio), available, ideal, PHOTO_GUTTER)) {
      const current = section.slice(layout.start, layout.end);
      const headings: PhotoHeading[] =
        grouping && layout.start === 0
          ? [{ date: day, index: 0, width: available, ids: groups.get(day) ?? [] }]
          : [];
      rows.push({
        assets: current,
        height: layout.height + (headings.length ? DATE_HEADING_HEIGHT : 0),
        imageHeight: layout.height,
        widths: current.map((asset) => layout.height * ratio(asset)),
        headings,
      });
    }
    start = end;
  }
  return rows;
}

export function assetDay(asset: AssetSummary, grouping: DateGrouping): string {
  const date = grouping === 'added' ? asset.addedAt : asset.date;
  return /^\d{4}-\d{2}-\d{2}/.test(date) ? date.slice(0, 10) : 'Undated';
}
const dayFormatter = new Intl.DateTimeFormat(undefined, {
  weekday: 'long',
  day: 'numeric',
  month: 'long',
  year: 'numeric',
  timeZone: 'UTC',
});
const compactDayFormatter = new Intl.DateTimeFormat(undefined, {
  day: 'numeric',
  month: 'short',
  year: 'numeric',
  timeZone: 'UTC',
});
function localDay(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}
export function dayLabel(day: string, now = new Date(), compact = false): string {
  if (day === 'Undated') return 'Unknown date';
  if (day === localDay(now)) return 'Today';
  if (day === localDay(new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1)))
    return 'Yesterday';
  const date = new Date(`${day}T12:00:00Z`);
  return Number.isNaN(date.getTime())
    ? 'Unknown date'
    : (compact ? compactDayFormatter : dayFormatter).format(date);
}

export function fileSize(value: string): string {
  const size = Number(value);
  if (!Number.isFinite(size)) return 'Unknown size';
  if (size < 1024) return `${size} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}
export function displayDate(value: string): string {
  // Capture-local timestamps with unknown offsets must not be shifted through Date/UTC.
  return value.slice(0, 19).replace('T', ' ');
}

const timelineMonthFormatter = new Intl.DateTimeFormat(undefined, {
  month: 'short',
  timeZone: 'UTC',
});

export function timelineDate(date: string) {
  const match = /^(\d{4})-(\d{2})-\d{2}/.exec(date);
  const month = Number(match?.[2]);
  if (!match || month < 1 || month > 12) return { year: 'Undated', label: 'Undated', position: 50 };
  const name = timelineMonthFormatter.format(new Date(`2000-${match[2]}-01T00:00:00Z`));
  return { year: match[1]!, label: `${name} ${match[1]}`, position: ((12 - month) / 12) * 100 };
}

export function normalizeFileExtensions(values: string[]): string[] {
  return Array.from(
    new Set(values.map((value) => value.trim().replace(/^\./, '').toLowerCase()).filter(Boolean)),
  );
}

export function galleryDateSections(rows: readonly PhotoRow[]) {
  const sections: {
    key: string;
    index: number;
    start: number;
    end: number;
    heading: PhotoHeading;
  }[] = [];
  const occurrences = new Map<string, number>();
  let start = 0;
  rows.forEach((row, index) => {
    const heading = row.headings[0];
    if (heading) {
      const previous = sections.at(-1);
      if (previous) previous.end = start;
      const occurrence = occurrences.get(heading.date) ?? 0;
      occurrences.set(heading.date, occurrence + 1);
      sections.push({ key: `${heading.date}:${occurrence}`, index, start, end: start, heading });
    }
    start += row.height + PHOTO_GUTTER;
  });
  const last = sections.at(-1);
  if (last) last.end = start;
  return sections;
}
