import type { DuplicateOptions } from '../../bindings';
import { InspectionChoiceCards, type InspectionChoice } from '@/components/InspectionChoiceCards';
import exactContent from '../../../assets/illustrations/duplicates/exact-content.png';
import properties from '../../../assets/illustrations/duplicates/properties.png';
import pixels from '../../../assets/illustrations/duplicates/pixels.png';

const modes: readonly InspectionChoice<DuplicateOptions['mode']>[] = [
  {
    value: 'exact_content',
    label: 'Identical file contents',
    description: 'Every byte must match, even when the filenames are different.',
    badge: 'Complete byte comparison',
    image: exactContent,
  },
  {
    value: 'properties',
    label: 'Matching file properties',
    description: 'Compare filenames, sizes, dates or other properties you choose.',
    badge: 'Candidates only',
    image: properties,
  },
  {
    value: 'normalized_pixels',
    label: 'Static PNG8 pixels',
    description: 'Compare supported pixel samples. The file contents may differ.',
    badge: 'Manual review only',
    image: pixels,
  },
];
const algorithms: readonly InspectionChoice<DuplicateOptions['algorithm']>[] = [
  { value: 'blake3', label: 'BLAKE3', description: 'Default content hash' },
  { value: 'sha256', label: 'SHA-256', description: 'Alternative content hash' },
  { value: 'sha1', label: 'SHA-1', description: 'Compatibility only' },
  { value: 'md5', label: 'MD5', description: 'Compatibility only' },
  { value: 'direct_bytes', label: 'Direct bytes', description: 'No full-file hash' },
];

export function DuplicateMatchOptions({
  options,
  onChange,
  disabled,
}: {
  options: DuplicateOptions;
  onChange: (options: DuplicateOptions) => void;
  disabled: boolean;
}) {
  return (
    <div className="duplicate-match-options">
      <InspectionChoiceCards
        label="Match mode"
        value={options.mode}
        choices={modes}
        presentation="illustrated"
        disabled={disabled}
        onChange={(mode) => onChange({ ...options, mode })}
      />
      <details className="duplicate-advanced-options">
        <summary>Advanced matching options</summary>
        <InspectionChoiceCards
          label="Content algorithm"
          value={options.algorithm}
          choices={algorithms}
          presentation="inline"
          disabled={disabled}
          onChange={(algorithm) => onChange({ ...options, algorithm })}
        />
        <label>
          <input
            type="checkbox"
            checked={options.excludeHardlinkedObjects}
            onChange={(event) =>
              onChange({ ...options, excludeHardlinkedObjects: event.target.checked })
            }
          />
          Exclude hard-linked objects from this search
        </label>
      </details>
    </div>
  );
}
