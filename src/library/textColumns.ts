export const TEXT_COLUMNS = [
  { id: 'modified', label: 'Modified Date', width: 170, sort: 'modified' },
  { id: 'size', label: 'Size', width: 80, sort: 'size' },
  { id: 'dimensions', label: 'Dimensions', width: 100, sort: null },
  { id: 'folder', label: 'Folder', width: 130, sort: null },
  { id: 'status', label: 'Status', width: 130, sort: null },
  { id: 'capture', label: 'Capture Date', width: 180, sort: 'capture' },
  { id: 'source', label: 'Date Source', width: 170, sort: null },
  { id: 'added', label: 'Added Date', width: 180, sort: 'added' },
  { id: 'format', label: 'File Format', width: 100, sort: null },
  { id: 'camera', label: 'Camera', width: 180, sort: null },
  { id: 'rating', label: 'Rating', width: 90, sort: null },
  { id: 'tags', label: 'Tags', width: 200, sort: null },
] as const;
export type TextColumn = (typeof TEXT_COLUMNS)[number]['id'];
export const DEFAULT_TEXT_COLUMNS: TextColumn[] = [
  'modified',
  'size',
  'dimensions',
  'folder',
  'status',
];
export function textListLayout(columns: readonly TextColumn[]) {
  const visible = TEXT_COLUMNS.filter((column) => columns.includes(column.id));
  return {
    visible,
    grid: `36px minmax(180px, 1fr) ${visible.map((column) => `${column.width}px`).join(' ')}`,
    minWidth: 216 + visible.reduce((sum, column) => sum + column.width, 0),
  };
}
