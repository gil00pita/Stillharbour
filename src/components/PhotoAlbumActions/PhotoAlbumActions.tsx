import { AlbumActions, usePhotoAlbumMenu, type OpenMenu } from './context';
import { useCallback, useContext, useEffect, useRef, useState, type ReactNode } from 'react';
import { Box, IconButton, Menu, Portal, useRecipe } from '@chakra-ui/react';
import type { AssetSummary, CatalogCollection, LibraryStatus } from '../../bindings';
import { metadata } from '../../metadata/ipc';
import { errorMessage } from '../../ipc';
import { InspectionDialog } from '../InspectionDialog';
import { AlbumEditor } from '../AlbumEditor';
import { Button } from '../ui/controls';
import { photoAlbumActionsRecipe } from './PhotoAlbumActions.recipe';

export function PhotoAlbumButton({ asset }: { asset: AssetSummary }) {
  const open = usePhotoAlbumMenu(asset);
  const state = useContext(AlbumActions);
  if (!open) return null;
  return (
    <IconButton
      type="button"
      className="photo-album-menu-button"
      aria-label={`Options for ${asset.name}`}
      aria-haspopup="menu"
      aria-expanded={state?.activeId === asset.id}
      onClick={open}
      variant="outline"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
        <circle cx="5" cy="12" r="1.5" />
        <circle cx="12" cy="12" r="1.5" />
        <circle cx="19" cy="12" r="1.5" />
      </svg>
    </IconButton>
  );
}
export function PhotoAlbumActions({
  status,
  children,
  onChanged,
}: {
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
      : event.currentTarget.querySelector<HTMLElement>('.photo-album-menu-button');
    const rect = event.currentTarget.getBoundingClientRect();
    setAnchor(
      event.type === 'contextmenu'
        ? { x: event.clientX, y: event.clientY }
        : { x: rect.left, y: rect.bottom },
    );
    setTarget(asset);
    setOpen(true);
  }, []);
  const styles = useRecipe({ recipe: photoAlbumActionsRecipe })();
  return (
    <AlbumActions.Provider value={{ show, activeId: open ? (target?.id ?? null) : null }}>
      {children}
      <span className="screen-reader-status" role="status">
        {notice}
      </span>
      <Menu.Root
        open={open}
        onOpenChange={(e) => {
          setOpen(e.open);
          if (!e.open) restore();
        }}
        anchorPoint={anchor}
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
      <div className="album-chooser">
        <p>
          Choose a custom album for <bdi>{asset.name}</bdi>.
        </p>
        {error && (
          <div role="alert">
            <p>{error}</p>
            <Button
              disabled={busy}
              onClick={() => {
                setError('');
                setAttempt((v) => v + 1);
              }}
            >
              Reload albums
            </Button>
          </div>
        )}
        {!collections && !error && <p role="status">Loading albums…</p>}
        {collections?.map((album) => (
          <Button key={album.id} disabled={busy} onClick={() => void add(album)}>
            {album.name}
            <span>{album.memberCount} photos</span>
          </Button>
        ))}
        {collections?.length === 0 && <p>No custom albums yet.</p>}
        <Button className="primary" disabled={busy} onClick={() => setCreating(true)}>
          + Create new album
        </Button>
        <p className="help">Tag albums follow photo tags automatically.</p>
        {busy && <p role="status">Adding photo…</p>}
      </div>
    </InspectionDialog>
  );
}
