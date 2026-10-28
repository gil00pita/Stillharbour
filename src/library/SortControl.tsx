const LIBRARY_SORTS = [
  { value: 'capture', label: 'Capture Date · newest first' },
  { value: 'name', label: 'Filename' },
  { value: 'size', label: 'Size · largest first' },
  { value: 'added', label: 'Recently added' },
  { value: 'modified', label: 'Modified Date · newest first' },
] as const;

export function SortControl({
  value,
  onChange,
  label = 'Sort',
  disabled = false,
}: {
  value: string;
  onChange: (sort: string) => void;
  label?: string;
  disabled?: boolean;
}) {
  return (
    <label>
      Sort
      <select
        aria-label={label}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      >
        {LIBRARY_SORTS.map((sort) => (
          <option key={sort.value} value={sort.value}>
            {sort.label}
          </option>
        ))}
      </select>
    </label>
  );
}
