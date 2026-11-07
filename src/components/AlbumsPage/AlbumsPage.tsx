import { CatalogActions } from '@/components/CatalogActions';
import { PageHeader } from '@/components/PageHeader';
import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { Box, useRecipe, Heading, Text, chakra } from '@chakra-ui/react';
import type {
  AlbumPage,
  AlbumSummary,
  AssetQuery,
  LibraryStatus,
  ThumbnailResult,
} from '../../bindings';
import { Button } from '@/components/Button';
import { SearchField } from '@/components/SearchField';
import { metadata } from '../../metadata/ipc';
import { errorMessage } from '../../ipc';
import { AlbumEditor } from '@/components/AlbumEditor';
import { TileImage } from '@/components/AssetTile';
import { previewKey, usePreviews } from '../../library/previews';
import { albumsPageRecipe } from './AlbumsPage.recipe';

export function AlbumsPage({
  status,
  jobsControl,
  onOpen,
  onRefresh,
  onBrowse,
}: {
  status: LibraryStatus;
  jobsControl?: ReactNode;
  onBrowse: (query: AssetQuery) => void;
  onOpen: (album: AlbumSummary) => void;
  onRefresh: () => Promise<void>;
}) {
  const [search, setSearch] = useState('');
  const [page, setPage] = useState<AlbumPage | null>(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(true);
  const [creating, setCreating] = useState(false);
  const [reload, setReload] = useState(0);
  const generation = useRef(0);
  const moreGate = useRef(false);
  const lastSearch = useRef('');
  const [notice, setNotice] = useState('');
  useEffect(() => {
    const epoch = ++generation.current;
    const timer = setTimeout(() => {
      setBusy(true);
      setError('');
      if (lastSearch.current !== search) {
        setPage(null);
        lastSearch.current = search;
      }
      void metadata({ kind: 'albums', search, offset: 0 }, 'albums')
        .then((value) => {
          if (epoch === generation.current) setPage(value);
        })
        .catch((e) => {
          if (epoch === generation.current) setError(errorMessage(e));
        })
        .finally(() => {
          if (epoch === generation.current) setBusy(false);
        });
    }, 150);
    return () => {
      clearTimeout(timer);
      generation.current = epoch + 1;
    };
  }, [search, status.revision, reload]);
  async function more() {
    if (page?.nextOffset == null || moreGate.current) return;
    moreGate.current = true;
    const epoch = generation.current;
    setBusy(true);
    setError('');
    try {
      const next = await metadata({ kind: 'albums', search, offset: page.nextOffset }, 'albums');
      if (epoch === generation.current) setPage({ ...next, items: [...page.items, ...next.items] });
    } catch (e) {
      if (epoch === generation.current) setError(errorMessage(e));
    } finally {
      moreGate.current = false;
      if (epoch === generation.current) setBusy(false);
    }
  }
  const [visible, setVisible] = useState<ReadonlySet<string>>(new Set());
  const visibility = useCallback(
    (id: string, showing: boolean) =>
      setVisible((previous) => {
        if (previous.has(id) === showing) return previous;
        const next = new Set(previous);
        if (showing) next.add(id);
        else next.delete(id);
        return next;
      }),
    [],
  );
  const refreshError = useCallback((e: unknown) => setError(errorMessage(e)), []);
  const previewError = useCallback(() => {}, []);
  const previews = usePreviews(
    (page?.items ?? [])
      .filter((a) => visible.has(a.id))
      .flatMap((a) => a.covers.map((c) => ({ assetId: c.id, revision: c.revision, edge: 256 }))),
    previewError,
    !creating,
  );
  const styles = useRecipe({ recipe: albumsPageRecipe })();
  return (
    <Box css={styles} className="albums-page">
      <PageHeader>
        <chakra.header
          className="toolbar"
          css={{
            flexShrink: '0',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '16px',
            minHeight: '64px',
            padding: '12px 24px',
            borderBottom: '1px solid {colors.border}',
            background: '{colors.bg.toolbar}',
            '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
          }}
        >
          <Heading as="h1" unstyled>
            Albums / Tags
          </Heading>
          <SearchField
            label="Search albums"
            placeholder="Search albums…"
            value={search}
            onChange={setSearch}
          />
          <Box
            className="header-actions"
            css={{ display: 'flex', alignItems: 'center', gap: '8px' }}
          >
            {jobsControl}
            <CatalogActions status={status} onBrowse={onBrowse} onRefresh={onRefresh} />
            <Button className="primary" aria-label="New album" onClick={() => setCreating(true)}>
              <Text as="span" aria-hidden="true">
                +
              </Text>{' '}
              <Text as="span" className="toolbar-action-label">
                New album
              </Text>
            </Button>
          </Box>
        </chakra.header>
      </PageHeader>
      <Box className="albums-content">
        <Box className="albums-intro">
          <Box>
            <Heading as="h2" unstyled>
              A place for every memory
            </Heading>
            <Text>Your photo tags, gathered into albums. Your own collections, too.</Text>
          </Box>
          {page && (
            <Text as="span">
              {page.total} {page.total === 1 ? 'album' : 'albums'}
            </Text>
          )}
        </Box>
        {error && (
          <Box role="alert">
            <Text>{error}</Text>
            <Button onClick={() => setReload((v) => v + 1)}>Retry albums</Button>
          </Box>
        )}
        {previews.error && (
          <Box role="alert">
            <Text>Album covers couldn’t be loaded. {previews.error}</Text>
            <Button onClick={previews.retry}>Retry covers</Button>
          </Box>
        )}
        {!page && busy && <Text role="status">Loading albums…</Text>}
        {page && (
          <Box className="album-grid">
            {page.items.map((album) => (
              <AlbumTile
                key={album.id}
                album={album}
                previews={previews.results}
                onVisibility={visibility}
                onOpen={() => onOpen(album)}
              />
            ))}
          </Box>
        )}
        {page && !page.items.length && (
          <Box className="albums-empty">
            <Heading as="h3" unstyled>
              {search ? 'No albums found' : 'Your collections start here'}
            </Heading>
            <Text>
              {search
                ? 'Try another album name.'
                : 'Create an album, or scan photos with embedded keywords to gather them here automatically.'}
            </Text>
            {!search && <Button onClick={() => setCreating(true)}>New album</Button>}
          </Box>
        )}
        {page?.nextOffset != null && (
          <Button disabled={busy} onClick={() => void more()}>
            {busy ? 'Loading…' : 'More albums'}
          </Button>
        )}
        <Text className="album-footnote">
          Albums keep your photos together without moving or changing the originals.
        </Text>
        <Text role="status">{notice}</Text>
      </Box>
      {creating && (
        <AlbumEditor
          status={status}
          onClose={() => setCreating(false)}
          onSaved={() => {
            setCreating(false);
            setReload((v) => v + 1);
            setNotice('Album created.');
            void onRefresh().catch(refreshError);
          }}
        />
      )}
    </Box>
  );
}
function AlbumTile({
  album,
  onOpen,
  previews,
  onVisibility,
}: {
  album: AlbumSummary;
  onOpen: () => void;
  previews: Record<string, ThumbnailResult>;
  onVisibility: (id: string, visible: boolean) => void;
}) {
  const element = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const node = element.current;
    if (!node) return;
    const observer = new IntersectionObserver(
      (entries) =>
        onVisibility(
          album.id,
          entries.some((e) => e.isIntersecting),
        ),
      { rootMargin: '200px' },
    );
    observer.observe(node);
    return () => {
      observer.disconnect();
      onVisibility(album.id, false);
    };
  }, [album.id, onVisibility]);
  return (
    <Button
      ref={element}
      type="button"
      className="album-tile"
      onClick={onOpen}
      aria-label={`${album.name}, ${album.photoCount} ${album.photoCount === 1 ? 'photo' : 'photos'}${album.fromTags ? ', From tags' : ''}`}
    >
      <Box className="album-cover">
        {album.covers.length ? (
          album.covers.map((asset) => (
            <TileImage
              key={asset.id}
              asset={asset}
              imageHeight={190}
              preview={
                previews[previewKey({ assetId: asset.id, revision: asset.revision, edge: 256 })]
              }
              edge={256}
              fit="cover"
              offline={false}
            />
          ))
        ) : (
          <Text as="span" className="album-no-cover">
            No photos yet
          </Text>
        )}
      </Box>
      <Box className="album-caption">
        <Heading as="h3" unstyled>
          <bdi>{album.name}</bdi>
        </Heading>
        {album.fromTags && (
          <Text as="span" className="tag-origin">
            From tags
          </Text>
        )}
      </Box>
      <Text>
        {album.photoCount} {album.photoCount === 1 ? 'photo' : 'photos'}
      </Text>
    </Button>
  );
}
