import { Field } from '@chakra-ui/react';
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
    <div className="view-switch" aria-label="Library view">
      {(['gallery', 'grid', 'list'] as const).map((view) => (
        <Button type="button" key={view} aria-pressed={mode === view} onClick={() => onMode(view)}>
          {view === 'gallery' ? 'Timeline' : view === 'grid' ? 'Uniform grid' : 'Text list'}
        </Button>
      ))}
      {mode !== 'list' && (
        <Field.Root asChild unstyled>
          <label className="thumbnail-control">
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
          </label>
        </Field.Root>
      )}
      {mode === 'grid' && (
        <Field.Root asChild unstyled>
          <label className="thumbnail-control">
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
          </label>
        </Field.Root>
      )}
    </div>
  );
}
