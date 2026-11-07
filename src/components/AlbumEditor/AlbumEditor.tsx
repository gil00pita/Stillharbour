import { useCallback, useId, useMemo, useRef, useState } from 'react';
import { Box, Field, useRecipe, Text, chakra } from '@chakra-ui/react';
import type { AssetQuery, CatalogCollection, LibraryStatus } from '../../bindings';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { InspectionDialog } from '@/components/InspectionDialog';
import { metadata } from '../../metadata/ipc';
import { errorMessage } from '../../ipc';
import { defaultQuery } from '../../library/model';
import { useAssetFeed } from '@/hooks/use-asset-feed';
import { AssetBrowser, type PhotoViewportHandle } from '@/components/AssetBrowser';
import { albumEditorRecipe } from './AlbumEditor.recipe';

export function AlbumEditor({
  status,
  album,
  initialIds = [],
  onClose,
  onSaved,
}: {
  status: LibraryStatus;
  album?: CatalogCollection | undefined;
  initialIds?: string[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const formId = useId();
  const [name, setName] = useState('');
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<ReadonlySet<string>>(() => new Set(initialIds));
  const [busy, setBusy] = useState(false);
  const saving = useRef(false);
  const [error, setError] = useState('');
  const onError = useCallback((e: unknown) => setError(errorMessage(e)), []);
  const query = useMemo<AssetQuery>(
    () => ({ ...defaultQuery, name: search, includeMissing: true }),
    [search],
  );
  const viewport = useRef<PhotoViewportHandle>(null);
  const remember = useCallback(() => viewport.current?.remember(), []);
  const feed = useAssetFeed(query, null, status.revision, onError, remember);
  const styles = useRecipe({ recipe: albumEditorRecipe })();
  function toggle(id: string) {
    setSelected((previous) => {
      const next = new Set(previous);
      if (next.has(id)) next.delete(id);
      else if (next.size < 500) next.add(id);
      return next;
    });
  }
  async function save() {
    if (saving.current) return;
    saving.current = true;
    setBusy(true);
    setError('');
    try {
      if (album)
        await metadata(
          {
            kind: 'collection_members',
            id: album.id,
            expected_version: album.version,
            expected_count: selected.size,
            selection: { kind: 'ids', ids: [...selected] },
            remove: false,
          },
          'membership',
        );
      else
        await metadata(
          { kind: 'create_album', name: name.trim(), asset_ids: [...selected] },
          'collection',
        );
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      saving.current = false;
      setBusy(false);
    }
  }
  return (
    <InspectionDialog
      busy={busy}
      footer={
        <>
          <Text as="span" role="status">
            {selected.size} photos selected <small>· up to 500 per save</small>
          </Text>
          <Button type="button" disabled={busy} onClick={onClose}>
            Cancel
          </Button>
          <Button
            className="primary"
            css={{
              marginTop: '28px',
              border: '0',
              background: '{colors.brand.solid}',
              color: '{colors.brand.contrast}',
              fontWeight: '600',
              fontSize: '13px',
              lineHeight: '20px',
              borderRadius: 'var(--control-radius)',
              padding: '12px 24px',
              minHeight: '44px',
              transition: 'background-color 140ms var(--ui-ease)',
              '&:hover:enabled': { background: '{colors.brand.hover}' },
            }}
            type="submit"
            form={formId}
            disabled={busy || (!album && !name.trim()) || Boolean(album && !selected.size)}
          >
            {busy ? 'Saving…' : album ? 'Add photos' : 'Create album'}
          </Button>
        </>
      }
      title={album ? `Add photos to ${album.name}` : 'Create an album'}
      onClose={() => {
        if (!saving.current) onClose();
      }}
    >
      <Box css={styles}>
        <chakra.form
          id={formId}
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <Text className="album-hint">
            {album
              ? 'Choose photos from your library to add to this album.'
              : 'Give it a name, then choose the photos that belong.'}{' '}
            Originals stay in place.
          </Text>
          {!album && (
            <Field.Root required invalid={Boolean(error)}>
              <Field.Label>Album name</Field.Label>
              <Input
                autoFocus
                required
                maxLength={128}
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={busy}
                placeholder="e.g. A day in the wild"
              />
              <Field.HelperText>Choose a unique name, including tag albums.</Field.HelperText>
            </Field.Root>
          )}
          <Field.Root>
            <Field.Label>Search library photos</Field.Label>
            <Input
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              disabled={busy}
            />
          </Field.Root>
          <Box className="album-picker" aria-label="Choose album photos">
            {feed.page?.assets.length ? (
              <AssetBrowser
                assets={feed.page.assets}
                startIndex={feed.page.startIndex ?? 0}
                mode="grid"
                display={{ size: 'compact', fit: 'cover' }}
                selected={selected}
                selectionFrozen={busy}
                activeId={null}
                onSelect={toggle}
                onInspect={toggle}
                onError={onError}
                onVisibleDate={() => {}}
                pageKey={feed.key}
                viewportHandle={viewport}
                onNearStart={feed.hasBefore ? feed.previous : undefined}
                onNearEnd={feed.hasAfter ? feed.next : undefined}
                pagingPaused={busy || feed.busy || Boolean(feed.error)}
                grouping={null}
              />
            ) : (
              <Text role="status">
                {feed.loading ? 'Loading photos…' : 'No matching photos in your library.'}
              </Text>
            )}
          </Box>
          {error && <Text role="alert">{error}</Text>}
          {feed.error && (
            <Button type="button" onClick={feed.retry}>
              Retry photos
            </Button>
          )}
        </chakra.form>
      </Box>
    </InspectionDialog>
  );
}
