import { chakra } from '@chakra-ui/react';
import { Field, Box } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import type { THUMBNAIL_SIZES, ViewMode } from '../../library/model';

export function BrowserControls({
  mode,
  size,
  fit,
  onMode,
  onSize,
  onFit,
}: {
  mode: ViewMode;
  size: keyof typeof THUMBNAIL_SIZES;
  fit: 'contain' | 'cover';
  onMode: (mode: ViewMode) => void;
  onSize: (size: keyof typeof THUMBNAIL_SIZES) => void;
  onFit: (fit: 'contain' | 'cover') => void;
}) {
  return (
    <Box
      className="view-switch"
      css={{
        display: 'flex',
        gap: '6px',
        marginBottom: '12px',
        flexShrink: '0',
        alignItems: 'center',
        flexWrap: 'wrap',
        '@media (max-width: 760px)': {
          gap: '4px',
          '& > button': { padding: '7px 8px !important', fontSize: '11px !important' },
        },
      }}
      aria-label="Library view"
    >
      {(['gallery', 'grid', 'list'] as const).map((view) => (
        <Button type="button" key={view} aria-pressed={mode === view} onClick={() => onMode(view)}>
          {view === 'gallery' ? 'Timeline' : view === 'grid' ? 'Uniform grid' : 'Text list'}
        </Button>
      ))}
      {mode !== 'list' && (
        <Field.Root asChild unstyled>
          <chakra.label
            className="thumbnail-control"
            css={{
              flexDirection: 'row',
              alignItems: 'center',
              fontSize: '12px',
              gap: '8px',
              marginLeft: 'auto',
              '& select': {
                width: 'auto',
                maxWidth: '160px',
                padding: '7px 36px 7px 12px',
                fontSize: '13px',
              },
              '@media (max-width: 760px)': { fontSize: '10px', '& select': { maxWidth: '160px' } },
            }}
          >
            <Field.Label asChild unstyled>
              <span>Thumbnail size</span>
            </Field.Label>
            <NativeSelect
              aria-label="Thumbnail size"
              value={size}
              onChange={(event) => {
                const value = event.target.value;
                if (value === 'compact' || value === 'comfortable' || value === 'large') {
                  onSize(value);
                }
              }}
            >
              <option value="compact">Compact</option>
              <option value="comfortable">Comfortable</option>
              <option value="large">Large</option>
            </NativeSelect>
          </chakra.label>
        </Field.Root>
      )}
      {mode === 'grid' && (
        <Field.Root asChild unstyled>
          <chakra.label
            className="thumbnail-control"
            css={{
              flexDirection: 'row',
              alignItems: 'center',
              fontSize: '12px',
              gap: '8px',
              marginLeft: 'auto',
              '& select': {
                width: 'auto',
                maxWidth: '160px',
                padding: '7px 36px 7px 12px',
                fontSize: '13px',
              },
              '@media (max-width: 760px)': { fontSize: '10px', '& select': { maxWidth: '160px' } },
            }}
          >
            <Field.Label asChild unstyled>
              <span>Image fit</span>
            </Field.Label>
            <NativeSelect
              aria-label="Grid image fit"
              value={fit}
              onChange={(event) => onFit(event.target.value === 'cover' ? 'cover' : 'contain')}
            >
              <option value="contain">Fit whole image</option>
              <option value="cover">Crop to fill</option>
            </NativeSelect>
          </chakra.label>
        </Field.Root>
      )}
    </Box>
  );
}
