import { useState } from 'react';
import type { ScanFilter } from '../bindings';
import { libraryData } from '../ipc';
import { normalizeFileExtensions } from './model';
import { scanTagFields, scanFilterFromForm } from './scanForm';
import { Switch } from '../components/Switch';
import { TagInput } from '../components/TagInput';
import { FileSizeField } from '../components/FileSizeField';
export function ScanOptions({
  filter,
  onChange,
  onError,
  disabled,
}: {
  filter: ScanFilter;
  disabled: boolean;
  onChange: (filter: ScanFilter) => void;
  onError: (error: unknown) => void;
}) {
  const [sample, setSample] = useState('Trips/harbour.jpg\n.hidden.jpg\nPrivate/family.png');
  const [preview, setPreview] = useState<string[]>([]);
  function update<K extends keyof ScanFilter>(key: K, value: ScanFilter[K]) {
    onChange({ ...filter, [key]: value });
    setPreview([]);
  }
  return (
    <details className="scan-options">
      <summary>
        <span>Advanced</span>
        <svg
          className="accordion-chevron"
          width="18"
          height="18"
          viewBox="0 0 24 24"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="m7 10 5 5 5-5"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </summary>
      <div className="advanced-content">
        <div className="scan-switches">
          <Switch
            label="Include hidden files"
            checked={filter.includeHiddenFiles}
            disabled={disabled}
            onChange={(value) => update('includeHiddenFiles', value)}
          />
          <Switch
            label="Include hidden folders"
            checked={filter.includeHiddenFolders}
            disabled={disabled}
            onChange={(value) => update('includeHiddenFolders', value)}
          />
        </div>
        <p className="help">
          Add one value per tag with Enter or Tab. Hidden files and hidden folders are controlled
          separately.
        </p>
        <div className="form-grid">
          {scanTagFields.map(({ key, label, placeholder }) => (
            <TagInput
              key={key}
              name={key}
              label={label}
              placeholder={placeholder}
              values={key === 'extensions' ? filter[key].map((value) => `.${value}`) : filter[key]}
              disabled={disabled}
              onChange={(values) =>
                update(key, key === 'extensions' ? normalizeFileExtensions(values) : values)
              }
            />
          ))}
          <FileSizeField
            name="minBytes"
            label="Minimum size"
            initialBytes={filter.minBytes}
            disabled={disabled}
            onChange={(value) => update('minBytes', value)}
          />
          <FileSizeField
            name="maxBytes"
            label="Maximum size"
            initialBytes={filter.maxBytes}
            disabled={disabled}
            onChange={(value) => update('maxBytes', value)}
          />
        </div>
        <p className="help">Size units use 1 KB = 1,000 bytes. Leave a size empty for no limit.</p>
        <p className="help">
          File Extensions match regardless of letter case. This list controls discovery; previews
          depend on format support.
        </p>
        <p className="help">
          Patterns use root-relative paths. Excluded folders win. Links, cloud placeholders and
          protected application folders are never opened.
        </p>
        <label>
          Try relative paths, one per line
          <textarea
            disabled={disabled}
            rows={3}
            value={sample}
            maxLength={4096}
            onChange={(event) => setSample(event.target.value)}
          />
        </label>
        <button
          type="button"
          disabled={disabled}
          onClick={(event) => {
            const form = event.currentTarget.form;
            if (form && form.reportValidity())
              void libraryData(
                {
                  kind: 'filter_preview',
                  filter: scanFilterFromForm(filter, form),
                  paths: sample.split('\n').filter(Boolean),
                },
                'filter_preview',
              )
                .then(setPreview)
                .catch(onError);
          }}
        >
          Preview filters
        </button>
        {preview.length > 0 && (
          <ul className="filter-preview">
            {Array.from(new Set(preview)).map((line) => (
              <li key={line}>
                <bdi>{line}</bdi>
              </li>
            ))}
          </ul>
        )}
      </div>
    </details>
  );
}
