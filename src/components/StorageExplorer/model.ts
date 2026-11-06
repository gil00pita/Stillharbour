import type { InspectionFinding } from '../../bindings';
import { byteValue } from '../../inspection/treemap';

export const categories = [
  'Videos',
  'Photos & RAW',
  'Audio',
  'Documents',
  'Archives',
  'Design files',
  'Other files',
] as const;
export type Category = (typeof categories)[number];
export const categoryColors: Record<Category, string> = {
  Videos: '#596e88',
  'Photos & RAW': '#667c58',
  Audio: '#896788',
  Documents: '#8c7550',
  Archives: '#756b91',
  'Design files': '#487f79',
  'Other files': '#646b70',
};
export const ageColors = {
  'Last 30 days': '#507969',
  'Last year': '#677b53',
  '1–3 years': '#8b7850',
  'Over 3 years': '#85606c',
  'Unknown date': '#646b70',
};
export type CameraRecord = { label: string | null; error?: string };
export type CameraRecords = Record<string, CameraRecord>;
export type Filters = {
  search: string;
  category: string;
  minimum: string;
  age: string;
  camera: string;
};
export const emptyFilters: Filters = { search: '', category: '', minimum: '', age: '', camera: '' };

export function categoryOf(file: InspectionFinding): Category {
  const extension = file.name.split('.').at(-1)?.toLowerCase() ?? '';
  if (/^(mp4|mov|m4v|avi|mkv|webm|mts|m2ts)$/.test(extension)) return 'Videos';
  if (
    /^(jpg|jpeg|heic|heif|png|gif|webp|tiff?|avif|dng|raw|cr[23]|nef|arw|raf|orf|rw2)$/.test(
      extension,
    )
  )
    return 'Photos & RAW';
  if (/^(mp3|wav|flac|aac|m4a|ogg|aiff)$/.test(extension)) return 'Audio';
  if (/^(pdf|txt|md|docx?|xlsx?|pptx?|csv|rtf|xmp)$/.test(extension)) return 'Documents';
  if (/^(zip|7z|rar|tar|gz|bz2)$/.test(extension)) return 'Archives';
  if (/^(psd|psb|ai|eps|sketch|fig|svg|indd)$/.test(extension)) return 'Design files';
  return 'Other files';
}
export function ageOf(file: InspectionFinding, now = Date.now()): keyof typeof ageColors {
  const time = file.filesystemModified ? Date.parse(file.filesystemModified) : NaN;
  if (!Number.isFinite(time) || time > now) return 'Unknown date';
  const days = (now - time) / 86_400_000;
  return days <= 30
    ? 'Last 30 days'
    : days <= 365
      ? 'Last year'
      : days <= 365 * 3
        ? '1–3 years'
        : 'Over 3 years';
}
export function cameraOf(file: InspectionFinding, cameras: CameraRecords): string {
  return file.isMedia ? cameras[file.id]?.label || 'Unknown camera' : 'Not camera media';
}
export function colorOf(file: InspectionFinding, mode: string, cameras: CameraRecords): string {
  if (mode === 'Age') return ageColors[ageOf(file)];
  if (mode !== 'Camera') return categoryColors[categoryOf(file)];
  const camera = cameraOf(file, cameras);
  if (camera === 'Unknown camera' || camera === 'Not camera media') return '#646b70';
  // Stable identity-derived colour; no inference about camera class or manufacturer.
  const hash = Array.from(camera).reduce((n, c) => (n * 31 + c.charCodeAt(0)) | 0, 0);
  return `hsl(${Math.abs(hash) % 360} 22% 38%)`;
}
export function totalBytes(files: InspectionFinding[]): string {
  return files.reduce((sum, file) => sum + byteValue(file.bytes), 0n).toString();
}
export function matches(
  file: InspectionFinding,
  filters: Filters,
  cameras: CameraRecords,
): boolean {
  const text = `${file.name} ${file.displayPath}`.toLocaleLowerCase();
  const camera = cameraOf(file, cameras);
  return (
    (!filters.search || text.includes(filters.search.toLocaleLowerCase())) &&
    (!filters.category || categoryOf(file) === filters.category) &&
    (!filters.minimum || byteValue(file.bytes) >= byteValue(filters.minimum)) &&
    (!filters.age || ageOf(file) === filters.age) &&
    (!filters.camera ||
      (filters.camera === 'Known camera'
        ? camera !== 'Unknown camera' && camera !== 'Not camera media'
        : camera === filters.camera))
  );
}
export function pathParts(file: InspectionFinding): string[] {
  // Display-only navigation. These labels never become filesystem/IPC inputs.
  return file.displayPath.replaceAll('\\', '/').split('/').filter(Boolean).slice(0, -1);
}
export function percentage(bytes: string, total: string): string {
  const denominator = byteValue(total);
  if (denominator === 0n || byteValue(bytes) === 0n) return '0%';
  const tenths = (byteValue(bytes) * 1000n) / denominator;
  return tenths === 0n ? '<0.1%' : `${Number(tenths) / 10}%`;
}

export function proportionalWidth(bytes: string, total: string): string {
  const denominator = byteValue(total);
  return denominator === 0n ? '0%' : `${(Number(byteValue(bytes)) / Number(denominator)) * 100}%`;
}

export function driveSegments(total: string | null, available: string | null) {
  if (total === null || available === null) return null;
  const capacity = byteValue(total);
  const free = byteValue(available);
  if (capacity === 0n || free > capacity) return null;
  return { capacity: total, used: String(capacity - free), free: available };
}
