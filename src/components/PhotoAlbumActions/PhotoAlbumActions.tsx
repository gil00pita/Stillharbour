import { AlbumActions, type OpenMenu, type PhotoSelection } from './context';
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Box, Menu, Portal, useRecipe, Text } from '@chakra-ui/react';
import type { AssetSummary, CatalogCollection, LibraryStatus } from '../../bindings';
import { metadata } from '../../metadata/ipc';
import { errorMessage } from '../../ipc';
import { InspectionDialog } from '@/components/InspectionDialog';
import { AlbumEditor } from '@/components/AlbumEditor';
import { Button } from '@/components/Button';
import { photoAlbumActionsRecipe } from './PhotoAlbumActions.recipe';

export function PhotoAlbumActions({
  status,
  children,
  onChanged,
  selection,
}: {
  selection?: PhotoSelection;
  status: LibraryStatus;
  children: ReactNode;
  onChanged: () => Promise<void>;
}) {
  const [target, setTarget] = useState<AssetSummary | null>(null);
  const [anchor, setAnchor] = useState({ x: 0, y: 0 });
  const [open, setOpen] = useState(false);
  const [choosing, setChoosing] = useState(false);
  const [notice, setNotice] = useState('');
  const opener = useRef<HTMLElement | null>(null);
  const restore = useCallback(() => {
    if (opener.current?.isConnected) opener.current.focus();
  }, []);
  const show = useCallback<OpenMenu>((asset, event) => {
    opener.current = event.currentTarget.matches('button')
      ? event.currentTarget
      : event.currentTarget.querySelector<HTMLElement>('.asset-open');
    const rect = event.currentTarget.getBoundingClientRect();
    setAnchor(
      event.type === 'contextmenu' && 'clientX' in event
        ? { x: event.clientX, y: event.clientY }
        : { x: rect.left, y: rect.bottom },
    );
    setTarget(asset);
    setOpen(true);
  }, []);
  // Floating UI observes the anchor by identity, so keep it stable for each context point.
  const virtualAnchor = useMemo(
    () => ({ getBoundingClientRect: () => new DOMRect(anchor.x, anchor.y, 0, 0) }),
    [anchor],
  );
  const styles = useRecipe({ recipe: photoAlbumActionsRecipe })();
  return (
    <AlbumActions.Provider value={{ show, selection }}>
      {children}
      <Text
        as="span"
        className="screen-reader-status"
        css={{
          position: 'absolute',
          width: '1px',
          height: '1px',
          overflow: 'hidden',
          clipPath: 'inset(50%)',
        }}
        role="status"
      >
        {notice}
      </Text>
      <Menu.Root
        key={`${anchor.x}:${anchor.y}`}
        open={open}
        onOpenChange={(e) => {
          setOpen(e.open);
          if (!e.open) restore();
        }}
        positioning={{
          strategy: 'fixed',
          placement: 'bottom-start',
          gutter: 0,
          getAnchorElement: () => virtualAnchor,
        }}
      >
        <Portal>
          <Menu.Positioner>
            <Menu.Content
              aria-label="Photo options"
              bg="bg.panel"
              color="fg"
              border="1px solid {colors.border.control}"
              borderRadius="control"
              p="1"
              minW="64"
              zIndex="popover"
            >
              {selection && (
                <Menu.Item
                  value="select-photo"
                  disabled={selection.disabled}
                  cursor="pointer"
                  p="3"
                  _highlighted={{ bg: 'bg.hover' }}
                  onClick={() => {
                    if (!target || selection.disabled) return;
                    selection.onSelect(target);
                    setOpen(false);
                  }}
                >
                  Select
                </Menu.Item>
              )}
              <Menu.Item
                value="add-to-album"
                cursor="pointer"
                p="3"
                _highlighted={{ bg: 'bg.hover' }}
                onClick={() => {
                  setOpen(false);
                  setChoosing(true);
                }}
              >
                Add to album
              </Menu.Item>
            </Menu.Content>
          </Menu.Positioner>
        </Portal>
      </Menu.Root>
      {choosing && target && (
        <AlbumActions.Provider value={null}>
          <Box css={styles}>
            <AlbumChooser
              status={status}
              asset={target}
              onClose={() => {
                setChoosing(false);
                restore();
              }}
              onSaved={() => {
                setChoosing(false);
                restore();
                setNotice('Photo added to album.');
                void onChanged().catch((e) => setNotice(errorMessage(e)));
              }}
            />
          </Box>
        </AlbumActions.Provider>
      )}
    </AlbumActions.Provider>
  );
}
function AlbumChooser({
  status,
  asset,
  onClose,
  onSaved,
}: {
  status: LibraryStatus;
  asset: AssetSummary;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [collections, setCollections] = useState<CatalogCollection[] | null>(null);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [attempt, setAttempt] = useState(0);
  const saving = useRef(false);
  useEffect(() => {
    let active = true;
    void metadata({ kind: 'collections' }, 'collections')
      .then((items) => {
        if (active) setCollections(items.filter((c) => c.kind === 'album' && !c.archived));
      })
      .catch((e) => {
        if (active) setError(errorMessage(e));
      });
    return () => {
      active = false;
    };
  }, [attempt]);
  async function add(album: CatalogCollection) {
    if (saving.current) return;
    saving.current = true;
    setBusy(true);
    setError('');
    try {
      await metadata(
        {
          kind: 'collection_members',
          id: album.id,
          expected_version: album.version,
          expected_count: 1,
          selection: { kind: 'ids', ids: [asset.id] },
          remove: false,
        },
        'membership',
      );
      onSaved();
    } catch (e) {
      setError(errorMessage(e));
      setAttempt((v) => v + 1);
    } finally {
      saving.current = false;
      setBusy(false);
    }
  }
  if (creating)
    return (
      <AlbumEditor
        status={status}
        initialIds={[asset.id]}
        onClose={() => setCreating(false)}
        onSaved={onSaved}
      />
    );
  return (
    <InspectionDialog
      busy={busy}
      title="Add to album"
      onClose={() => {
        if (!saving.current) onClose();
      }}
    >
      <Box className="album-chooser">
        <Text>
          Choose a custom album for <bdi>{asset.name}</bdi>.
        </Text>
        {error && (
          <Box role="alert">
            <Text>{error}</Text>
            <Button
              disabled={busy}
              onClick={() => {
                setError('');
                setAttempt((v) => v + 1);
              }}
            >
              Reload albums
            </Button>
          </Box>
        )}
        {!collections && !error && <Text role="status">Loading albums…</Text>}
        {collections?.map((album) => (
          <Button key={album.id} disabled={busy} onClick={() => void add(album)}>
            {album.name}
            <Text as="span">{album.memberCount} photos</Text>
          </Button>
        ))}
        {collections?.length === 0 && <Text>No custom albums yet.</Text>}
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
          disabled={busy}
          onClick={() => setCreating(true)}
        >
          + Create new album
        </Button>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Tag albums follow photo tags automatically.
        </Text>
        {busy && <Text role="status">Adding photo…</Text>}
      </Box>
    </InspectionDialog>
  );
}
