import { chakra } from '@chakra-ui/react';
import { Field, Box, List, Text } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Textarea } from '@/components/Textarea';
import { useState } from 'react';
import type { ScanFilter } from '../../bindings';
import { libraryData } from '../../ipc';
import { normalizeFileExtensions } from '../../library/model';
import { scanTagFields, scanFilterFromForm } from '../../library/scanForm';
import { Switch } from '@/components/Switch';
import { TagInput } from '@/components/TagInput';
import { FileSizeField } from '@/components/FileSizeField';
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
    <chakra.details
      className="scan-options"
      css={{
        flexBasis: '100%',
        '& textarea': { margin: '0 0 10px' },
        border: '1px solid {colors.border}',
        borderRadius: 'var(--control-radius)',
        '& > summary': {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          width: '100%',
          minHeight: '46px',
          padding: '12px 16px',
          listStyle: 'none',
        },
        '& > summary::-webkit-details-marker': { display: 'none' },
        '& > summary::marker': { content: "''" },
        '& > summary:hover': { background: '{colors.bg.panel}' },
        '&[open] > summary': { borderBottom: '1px solid {colors.border}' },
        '&[open] .accordion-chevron': { transform: 'rotate(180deg)' },
      }}
    >
      <summary>
        <Text as="span">Advanced</Text>
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
      <Box className="advanced-content" css={{ padding: '8px 16px 20px' }}>
        <Box
          className="scan-switches"
          css={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
            gap: '16px',
            margin: '12px 0',
            '@media (max-width: 760px)': { gridTemplateColumns: '1fr', gap: '2px' },
          }}
        >
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
        </Box>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Add one value per tag with Enter or Tab. Hidden files and hidden folders are controlled
          separately.
        </Text>
        <Box
          className="form-grid"
          css={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
            gap: '16px',
            width: '100%',
            '@media (max-width: 600px)': { gridTemplateColumns: '1fr' },
          }}
        >
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
        </Box>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Size units use 1 KB = 1,000 bytes. Leave a size empty for no limit.
        </Text>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          File Extensions match regardless of letter case. This list controls discovery; previews
          depend on format support.
        </Text>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Patterns use root-relative paths. Excluded folders win. Links, cloud placeholders and
          protected application folders are never opened.
        </Text>
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Try relative paths, one per line</span>
            </Field.Label>
            <Textarea
              disabled={disabled}
              rows={3}
              value={sample}
              maxLength={4096}
              onChange={(event) => setSample(event.target.value)}
            />
          </label>
        </Field.Root>
        <Button
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
        </Button>
        {preview.length > 0 && (
          <List.Root
            unstyled
            className="filter-preview"
            css={{
              fontSize: '12px',
              paddingLeft: '18px',
              lineHeight: '1.7',
              overflowWrap: 'anywhere',
            }}
          >
            {Array.from(new Set(preview)).map((line) => (
              <List.Item unstyled key={line}>
                <bdi>{line}</bdi>
              </List.Item>
            ))}
          </List.Root>
        )}
      </Box>
    </chakra.details>
  );
}
