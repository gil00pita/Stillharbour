import { IconButton, Text } from '@chakra-ui/react';
import { useState } from 'react';
import type { AssetSummary } from '../../../bindings';

export function ThumbnailFavorite({
  asset,
  disabled,
  onSave,
}: {
  asset: AssetSummary;
  disabled: boolean;
  onSave: (id: string, favorite: boolean) => Promise<void>;
}) {
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  return (
    <>
      <IconButton
        className="photo-favorite"
        aria-label={`${asset.favorite ? 'Unfavorite' : 'Favorite'} ${asset.name}`}
        aria-pressed={asset.favorite}
        disabled={saving || disabled}
        title={error ?? (asset.favorite ? 'Remove favorite' : 'Add favorite')}
        position="absolute"
        top="2"
        right="2"
        width="control"
        height="control"
        minHeight="control"
        padding="0"
        border="0"
        borderRadius="control"
        bg="bg.elevated"
        color="fg"
        transition="opacity 150ms cubic-bezier(0.16, 1, 0.3, 1)"
        _motionReduce={{ transition: 'none' }}
        onClick={() => {
          setSaving(true);
          setError(null);
          void onSave(asset.id, !asset.favorite)
            .catch(() => setError('Favorite couldn’t be saved. Try again.'))
            .finally(() => setSaving(false));
        }}
      >
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill={asset.favorite ? 'currentColor' : 'none'}
          aria-hidden="true"
        >
          <path
            d="M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1.1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinejoin="round"
          />
        </svg>
      </IconButton>
      {error && (
        <Text
          role="alert"
          fontSize="xs"
          position="absolute"
          bottom="0"
          insetInline="0"
          margin="0"
          padding="2"
          maxHeight="full"
          overflowY="auto"
          bg="bg.elevated"
          color="fg"
        >
          {error}
        </Text>
      )}
    </>
  );
}
