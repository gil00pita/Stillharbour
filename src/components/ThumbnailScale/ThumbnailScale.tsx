import { Field, Flex } from '@chakra-ui/react';
import { Input } from '@/components/Input';
import type { GalleryDisplay } from '@/components/FilterPanel';

const sizes = ['compact', 'comfortable', 'large'] as const;

export function ThumbnailScale({
  size,
  onChange,
  disabled = false,
}: {
  size: GalleryDisplay['size'];
  onChange: (size: GalleryDisplay['size']) => void;
  disabled?: boolean;
}) {
  return (
    <Field.Root width="full" disabled={disabled}>
      <Field.Label>Thumbnail size</Field.Label>
      <Flex align="center" gap="2" width="full" color="fg.muted">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M2 2h4v4H2Zm8 0h4v4h-4ZM2 10h4v4H2Zm8 0h4v4h-4Z" stroke="currentColor" />
        </svg>
        <Input
          type="range"
          aria-label="Thumbnail scale"
          aria-valuetext={size}
          disabled={disabled}
          min={0}
          max={2}
          step={1}
          value={sizes.indexOf(size)}
          onChange={(event) => onChange(sizes[Number(event.target.value)] ?? 'comfortable')}
          css={{
            width: 'full',
            minWidth: 0,
            height: 'control',
            margin: 0,
            padding: 0,
            accentColor: 'brand.solid',
            background: 'transparent',
            border: 0,
          }}
        />
      </Flex>
      <Field.HelperText>{size.charAt(0).toUpperCase() + size.slice(1)}</Field.HelperText>
    </Field.Root>
  );
}
