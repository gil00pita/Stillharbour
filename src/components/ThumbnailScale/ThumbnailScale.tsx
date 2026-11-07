import { Field, Flex, chakra } from '@chakra-ui/react';
import type { GalleryDisplay } from '@/components/FilterPanel';

const sizes = ['compact', 'comfortable', 'large'] as const;

export function ThumbnailScale({
  size,
  onChange,
}: {
  size: GalleryDisplay['size'];
  onChange: (size: GalleryDisplay['size']) => void;
}) {
  return (
    <Field.Root width="24" flexShrink="0" hideBelow="lg">
      <Field.Label srOnly>Thumbnail scale</Field.Label>
      <Flex align="center" gap="2" width="full" color="fg.muted">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path d="M2 2h4v4H2Zm8 0h4v4h-4ZM2 10h4v4H2Zm8 0h4v4h-4Z" stroke="currentColor" />
        </svg>
        <chakra.input
          type="range"
          aria-label="Thumbnail scale"
          aria-valuetext={size}
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
    </Field.Root>
  );
}
