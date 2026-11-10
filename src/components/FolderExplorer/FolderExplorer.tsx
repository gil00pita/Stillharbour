import { PageHeader } from '@/components/PageHeader';
import { Box, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { SearchField } from '@/components/SearchField';
import type { ReactNode } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import type {
  AnnotationPatch,
  AssetSummary,
  DirectoryEntry,
  LibraryStatus,
  LibraryRoot,
  DirectoryPage,
} from '../../bindings';
import { libraryData } from '../../ipc';
import { defaultQuery } from '../../library/model';
import { PhotoViewer } from '@/components/PhotoViewer';
import { photoOrigin, type PhotoOrigin } from '../../library/viewerMotion';
import { restoreAssetFocus } from '../../library/focus';
import { ExplorerCanvas, type ExplorerItem } from '@/components/ExplorerCanvas';
import { FileDetails } from '@/components/FileDetails';
import { useDirectoryPage, type FolderLocation } from '@/hooks/use-directory-page';
const HOME: FolderLocation = { rootId: null, directoryId: 'd', cursor: null, search: '' };
type Position = { location: FolderLocation; offset: number; focus: string | null };
type FolderExplorerProps = {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  status: LibraryStatus;
  onError: (error: unknown) => void;
  onRefresh: () => Promise<void>;
};

export function FolderExplorer({
  jobsControl,
  navigationControl,
  status,
  onError,
  onRefresh,
}: FolderExplorerProps) {
  const [location, setLocation] = useState(HOME);
  const [history, setHistory] = useState<Position[]>([]);
  const [mode, setMode] = useState<'icons' | 'details'>('icons');
  const [viewer, setViewer] = useState<AssetSummary | null>(null);
  const [file, setFile] = useState<AssetSummary | null>(null);
  const viewport = useRef<HTMLDivElement>(null);
  const restore = useRef<Position | null>(null);
  const opener = useRef<HTMLButtonElement | null>(null);
  const viewerOffset = useRef(0);
  const [origin, setOrigin] = useState<PhotoOrigin | undefined>(undefined);
  const root = status.roots.find((root) => root.id === location.rootId);
  const effectiveLocation = useMemo(() => (root ? location : HOME), [root, location]);
  const browse = useDirectoryPage(effectiveLocation, status.revision);
  const page = browse.page;
  const query = useMemo(
    () => ({
      ...defaultQuery,
      rootId: root?.id ?? null,
      directoryId: page?.directory.id ?? 'd',
      name: location.search,
      sort: 'name',
      includeMissing: false,
    }),
    [root?.id, page?.directory.id, location.search],
  );
  const items = useMemo<ExplorerItem[]>(
    () =>
      root
        ? (page?.entries ?? [])
        : status.roots
            .filter((value) =>
              value.label.toLocaleLowerCase().includes(location.search.toLocaleLowerCase()),
            )
            .map((value) => ({ kind: 'root', root: value })),
    [root, page, status.roots, location.search],
  );
  const photos = useMemo(
    () =>
      page?.entries
        .filter(
          (entry): entry is Extract<DirectoryEntry, { kind: 'file' }> =>
            entry.kind === 'file' && entry.media,
        )
        .map((entry) => entry.asset) ?? [],
    [page],
  );
  function navigate(next: FolderLocation, focus: string | null = null) {
    setHistory((previous) => [
      ...previous.slice(-63),
      { location, offset: viewport.current?.scrollTop ?? 0, focus },
    ]);
    restore.current = { location: next, offset: 0, focus: null };
    setLocation(next);
  }
  function back() {
    const previous = history.at(-1);
    if (!previous) return;
    restore.current = previous;
    setHistory(history.slice(0, -1));
    setLocation(previous.location);
  }
  useEffect(() => {
    if (browse.loading || !restore.current) return;
    const position = restore.current;
    restore.current = null;
    if (viewport.current) viewport.current.scrollTop = position.offset;
    if (position.focus) restoreAssetFocus(position.focus, null, position.offset);
  }, [browse.loading, page, items]);
  function open(entry: ExplorerItem, button: HTMLButtonElement) {
    if (entry.kind === 'root')
      navigate(
        { rootId: entry.root.id, directoryId: 'd', cursor: null, search: '' },
        entry.root.id,
      );
    else if (entry.kind === 'folder')
      navigate(
        { ...location, directoryId: entry.directory.id, cursor: null, search: '' },
        entry.directory.id,
      );
    else {
      opener.current = button;
      setOrigin(photoOrigin(button));
      viewerOffset.current = viewport.current?.scrollTop ?? 0;
      if (entry.media) setViewer(entry.asset);
      else setFile(entry.asset);
    }
  }
  async function annotate(patch: AnnotationPatch, id?: string) {
    const assetId = id ?? viewer?.id;
    if (!assetId) throw new Error('Select a catalog item first.');
    await libraryData(
      { kind: 'annotate', selection: { kind: 'ids', ids: [assetId] }, patch },
      'updated',
    );
    await onRefresh();
  }
  const parent = page?.ancestors.at(-1);
  return (
    <Box
      className="library-page"
      css={{
        display: 'flex',
        flexDirection: 'column',
        flex: '1',
        minWidth: '0',
        minHeight: '0',
        position: 'relative',
        overflow: 'hidden',
        isolation: 'isolate',
      }}
    >
      <Box
        className="library-underlay"
        css={{ display: 'flex', flexDirection: 'column', flex: '1', minWidth: '0', minHeight: '0' }}
        inert={Boolean(viewer)}
        aria-hidden={Boolean(viewer)}
      >
        <PageHeader>
          <chakra.header
            className="toolbar explorer-toolbar"
            css={{
              flexShrink: '0',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              gap: '12px',
              minHeight: '64px',
              padding: '12px 24px',
              borderBottom: '1px solid {colors.border}',
              background: '{colors.bg.toolbar}',
              '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
              '@media (max-width: 760px)': {
                flexWrap: 'wrap',
                '& .fixed-search': { minWidth: '140px' },
              },
            }}
          >
            {navigationControl}
            <Heading as="h1" unstyled>
              Folders
            </Heading>
            <SearchField
              maxLength={256}
              label={root ? 'Filter files in this folder' : 'Search libraries'}
              placeholder={root ? 'Filter files in this folder…' : 'Search libraries…'}
              value={location.search}
              onChange={(search) => {
                restore.current = { location, offset: 0, focus: null };
                setLocation({ ...location, cursor: null, search });
              }}
            />
            <Box
              className="explorer-views"
              css={{ display: 'flex', gap: '6px' }}
              aria-label="Folder view"
            >
              <Button aria-pressed={mode === 'icons'} onClick={() => setMode('icons')}>
                Icons
              </Button>
              <Button aria-pressed={mode === 'details'} onClick={() => setMode('details')}>
                Details
              </Button>
            </Box>
            {jobsControl}
          </chakra.header>
        </PageHeader>
        <Box
          className="folder-explorer"
          css={{
            display: 'flex',
            flexDirection: 'column',
            flex: '1',
            minHeight: '0',
            minWidth: '0',
            padding: '16px 24px 12px',
            '@media (max-width: 760px)': { padding: '12px 16px' },
          }}
        >
          <Box
            className="explorer-navigation"
            css={{
              display: 'flex',
              alignItems: 'center',
              gap: '8px',
              flexWrap: 'wrap',
              '& nav': {
                margin: '0',
                padding: '0',
                display: 'flex',
                alignItems: 'center',
                flexWrap: 'nowrap',
                overflowX: 'auto',
                maxWidth: '100%',
                whiteSpace: 'nowrap',
                gap: '6px',
                minWidth: '0',
                fontSize: '12px',
              },
              '& nav > span': {
                flexShrink: '0',
                display: 'flex',
                alignItems: 'center',
                gap: '6px',
                minWidth: '0',
              },
              '& bdi': { overflowWrap: 'anywhere' },
              '@media (max-width: 760px)': { '& nav': { flexBasis: '100%' } },
              '& nav button': {
                maxWidth: '200px',
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                flexShrink: '0',
              },
              "& [aria-current='location'] bdi": {
                maxWidth: '260px',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              },
            }}
          >
            <Button onClick={back} disabled={!history.length}>
              Back
            </Button>
            <Button
              onClick={() =>
                navigate(
                  parent ? { ...location, directoryId: parent.id, cursor: null, search: '' } : HOME,
                )
              }
              disabled={!root}
            >
              Up
            </Button>
            <chakra.nav aria-label="Folder path">
              <Button onClick={() => navigate(HOME)} disabled={!root}>
                Libraries
              </Button>
              {root && (
                <>
                  {(page?.ancestors ?? []).map((directory) => (
                    <Text as="span" key={directory.id}>
                      <Text as="span" aria-hidden="true">
                        /
                      </Text>
                      <Button
                        title={directory.name}
                        onClick={() =>
                          navigate({
                            ...location,
                            directoryId: directory.id,
                            cursor: null,
                            search: '',
                          })
                        }
                      >
                        <bdi>{directory.name}</bdi>
                      </Button>
                    </Text>
                  ))}
                  <Text as="span" aria-current="location">
                    <Text as="span" aria-hidden="true">
                      /
                    </Text>
                    <bdi>{page?.directory.name ?? root.label}</bdi>
                  </Text>
                </>
              )}
            </chakra.nav>
          </Box>
          <FolderStatus root={root} page={page} libraryCount={status.roots.length} />
          {browse.error && (
            <Text
              role="alert"
              className="error"
              css={{
                border: '1px solid {colors.border.control}',
                borderRadius: 'var(--control-radius)',
                background: '{colors.danger.bg}',
                color: '{colors.danger.fg}',
                padding: '16px 20px',
                margin: '24px 0 0',
                fontSize: '13px',
                lineHeight: '1.6',
                '& p': { margin: '0' },
                '& button': {
                  marginTop: '12px',
                  border: '1px solid currentColor',
                  background: 'transparent',
                  color: 'inherit',
                  borderRadius: '4px',
                  padding: '7px 12px',
                },
              }}
            >
              {browse.error} <Button onClick={browse.retry}>Retry folder</Button>
            </Text>
          )}
          {browse.loading && !page ? (
            <Text role="status">Loading indexed contents…</Text>
          ) : browse.error && !page ? (
            <Box
              className="empty-results"
              css={{
                flex: '1',
                textAlign: 'center',
                alignContent: 'center',
                padding: '50px 20px',
                '& h2': { fontSize: '22px' },
                '& p': { fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '1.6' },
              }}
            >
              <Heading as="h2" unstyled>
                Folder could not be loaded
              </Heading>
              <Text>Retry, or choose another folder.</Text>
            </Box>
          ) : items.length ? (
            <ExplorerCanvas
              items={items}
              mode={mode}
              viewport={viewport}
              root={root}
              paused={Boolean(viewer || file)}
              onOpen={open}
              onError={onError}
            />
          ) : (
            <Box
              className="empty-results"
              css={{
                flex: '1',
                textAlign: 'center',
                alignContent: 'center',
                padding: '50px 20px',
                '& h2': { fontSize: '22px' },
                '& p': { fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '1.6' },
              }}
            >
              <Heading as="h2" unstyled>
                {root
                  ? location.search
                    ? 'No matching indexed files'
                    : 'No indexed items here'
                  : location.search
                    ? 'No matching libraries'
                    : 'No libraries added'}
              </Heading>
              <Text>
                {root
                  ? 'Choose another folder, or check its scan settings.'
                  : 'Add a photo folder in Settings to browse its indexed contents.'}
              </Text>
            </Box>
          )}
          {root && page && (
            <Box
              className="explorer-pagination"
              css={{
                display: 'flex',
                alignItems: 'center',
                gap: '8px',
                paddingTop: '12px',
                fontSize: '11px',
                color: '{colors.fg.muted}',
                flexWrap: 'wrap',
                '& > span': { marginRight: 'auto' },
              }}
            >
              <Text as="span">
                {page.entries.length.toLocaleString()} shown · {page.total.toLocaleString()} indexed
                items
              </Text>
              <Button
                disabled={browse.loading || !page.previousCursor}
                onClick={() => {
                  restore.current = { location, offset: 0, focus: null };
                  setLocation({ ...location, cursor: page.previousCursor });
                }}
              >
                Previous items
              </Button>
              <Button
                disabled={browse.loading || !page.nextCursor}
                onClick={() => {
                  restore.current = { location, offset: 0, focus: null };
                  setLocation({ ...location, cursor: page.nextCursor });
                }}
              >
                Next items
              </Button>
            </Box>
          )}
        </Box>
      </Box>
      {viewer && (
        <PhotoViewer
          initial={viewer}
          origin={origin}
          pageAssets={photos}
          query={query}
          status={status}
          onAnnotate={annotate}
          onError={onError}
          onClose={() => {
            setViewer(null);
            restoreAssetFocus(viewer.id, opener.current, viewerOffset.current);
          }}
        />
      )}
      {file && (
        <FileDetails
          asset={file}
          root={root}
          onClose={() => {
            setFile(null);
            restoreAssetFocus(file.id, opener.current, viewerOffset.current);
          }}
        />
      )}
    </Box>
  );
}

function FolderStatus({
  root,
  page,
  libraryCount,
}: {
  root: LibraryRoot | undefined;
  page: DirectoryPage | null;
  libraryCount: number;
}) {
  return (
    <>
      <Text
        className="explorer-status"
        css={{ fontSize: '12px', color: '{colors.fg.muted}', margin: '14px 0 4px' }}
        role="status"
      >
        {root
          ? `${root.label} · ${root.availability.replaceAll('_', ' ')} · ${page?.total.toLocaleString() ?? '…'} indexed items in this folder`
          : `${libraryCount} indexed libraries`}
      </Text>
      {root && !['online', 'scanning'].includes(root.availability) && (
        <Text
          className="availability-note"
          css={{
            background: '{colors.bg.subtle}',
            color: '{colors.fg.secondary}',
            padding: '12px',
            fontSize: '12px',
            lineHeight: '1.5',
          }}
        >
          Originals are {root.availability.replaceAll('_', ' ')}. Showing retained catalog contents
          and available cached previews.
        </Text>
      )}
      {root && (root.availability === 'scanning' || page?.directory.state === 'incomplete') && (
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Indexing is incomplete. These catalog contents may be partial.
        </Text>
      )}
      {page?.directory.state === 'missing' && (
        <Text
          className="availability-note"
          css={{
            background: '{colors.bg.subtle}',
            color: '{colors.fg.secondary}',
            padding: '12px',
            fontSize: '12px',
            lineHeight: '1.5',
          }}
        >
          This folder is no longer at this location. Go Up to see current folders; its changes
          remain in Jobs.
        </Text>
      )}
      {page?.directory.state === 'retained' && (
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          This folder was not seen in the latest scan. Showing retained catalog contents.
        </Text>
      )}
      <Text
        className="explorer-help help"
        css={{
          fontSize: '13px',
          color: '{colors.fg.muted}',
          lineHeight: '20px',
          margin: '4px 0 12px',
        }}
      >
        Only items allowed by your scan settings are indexed. Filename filtering keeps folders
        visible.
      </Text>
    </>
  );
}
