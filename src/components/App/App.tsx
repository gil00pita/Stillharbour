import { PageHeader } from '@/components/PageHeader';
import { metadata } from '../../metadata/ipc';
import { AlbumsPage } from '@/components/AlbumsPage';
import { AlbumEditor } from '@/components/AlbumEditor';
import type { AlbumSummary, CatalogCollection } from '../../bindings';
import { Button } from '@/components/Button';
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { useFoundation } from '@/hooks/use-foundation';
import { usePreferences } from '@/hooks/use-preferences';
import { SettingsScreen } from '@/components/SettingsScreen';
import type {
  AssetQuery,
  DirectorySelection,
  LibraryRoot,
  RootMode,
  ScanFilter,
} from '../../bindings';
import { MapScreen } from '@/components/MapScreen';
import { initialMapState, type MapBrowseState } from '../../map/state';
import { JobsButton } from '@/components/JobsButton';
import { JobsPage } from '@/components/JobsPage';
import { LibraryScreen, type Section } from '@/components/LibraryScreen';
import { FolderExplorer } from '@/components/FolderExplorer';
import { InspectionScreen, type InspectionSection } from '@/components/InspectionScreen';
import { RootSetup } from '@/components/RootSetup';
import logo from '../../../assets/branding/stillharbour-logo.svg';
import { libraryData } from '../../ipc';
import { BackupScreen } from '@/components/BackupScreen';
import { SyncScreen } from '@/components/SyncScreen';
import { StructureFormatter } from '@/components/StructureFormatter';
import { useCatalogSwitchState } from '../../backup/catalogSwitchState';
import { CatalogSwitchStopped } from '@/components/CatalogSwitchStopped';
import { NotesDraftContext, type NotesDraft } from '../../metadata/notesDrafts';
import { ErrorToast } from '@/components/ErrorToast';
import { Box, Text, Heading, Image, chakra } from '@chakra-ui/react';

type AppSection =
  Section | 'Settings' | InspectionSection | 'Backup' | 'Sync' | 'Structure Formater' | 'Map';
type LibraryLaunch = { query: AssetQuery; selectedId: string | null; generation: number };
export default function App() {
  const stopped = useCatalogSwitchState();
  return stopped ? <CatalogSwitchStopped state={stopped} /> : <ConnectedApplication />;
}

function ConnectedApplication() {
  const app = useFoundation();
  const {
    foundation,
    status,
    selected,
    setSelected,
    setSettingsId,
    busy,
    error,
    setError,
    notice,
    refresh,
    chooseFolder,
    reconnect,
  } = app;
  const settings = usePreferences();
  const preferences = settings.preferences;
  const [section, setSection] = useState<AppSection>('Photos');
  const [jobsOpen, setJobsOpen] = useState(false);
  const jobsButton = useRef<HTMLButtonElement>(null);
  const returnFromJobs = useRef(false);
  const main = useRef<HTMLElement>(null);
  const workspaceScroll = useRef<{ element: HTMLElement; top: number; left: number }[]>([]);
  useLayoutEffect(() => {
    if (main.current) main.current.scrollTop = 0;
    if (!jobsOpen)
      for (const position of workspaceScroll.current) {
        if (!position.element.isConnected) continue;
        position.element.scrollTop = position.top;
        position.element.scrollLeft = position.left;
      }
  }, [jobsOpen]);
  useEffect(() => {
    if (jobsOpen || !returnFromJobs.current) return;
    returnFromJobs.current = false;
    // Native pointer activation must finish before the unmounted Back button
    // yields focus to the persistent navigation entry.
    const timer = setTimeout(() => jobsButton.current?.focus(), 0);
    return () => clearTimeout(timer);
  }, [jobsOpen]);
  const [mapState, setMapState] = useState(initialMapState);
  const [libraryLaunch, setLibraryLaunch] = useState<LibraryLaunch | null>(null);
  const [navigationOverride, setNavigationOverride] = useState<boolean | null>(null);
  const collapsed = navigationOverride ?? preferences?.navigationCollapsed ?? false;
  const hasLibrary = Boolean(status?.roots.length);
  const jobsControl = null;
  const configuring = Boolean(selected) || (!hasLibrary && section !== 'Settings');
  return (
    <Box
      className={`app-shell ${hasLibrary ? 'has-library' : ''} ${collapsed ? 'navigation-collapsed' : ''}`}
    >
      <Sidebar
        jobsOpen={jobsOpen}
        jobsControl={
          <JobsButton
            status={status}
            current={jobsOpen}
            buttonRef={jobsButton}
            onOpen={() => {
              if (!jobsOpen && main.current)
                workspaceScroll.current = [
                  main.current,
                  ...main.current.querySelectorAll<HTMLElement>('*'),
                ]
                  .filter((element) => element.scrollTop !== 0 || element.scrollLeft !== 0)
                  .map((element) => ({
                    element,
                    top: element.scrollTop,
                    left: element.scrollLeft,
                  }));
              setJobsOpen(true);
            }}
          />
        }
        hasLibrary={hasLibrary}
        version={foundation?.version}
        section={section}
        collapsed={collapsed}
        onToggleNavigation={() => setNavigationOverride(!collapsed)}
        onNavigate={(item) => {
          returnFromJobs.current = false;
          workspaceScroll.current = [];
          setJobsOpen(false);
          setLibraryLaunch(null);
          setSection(item);
          setSelected(null);
          setSettingsId(null);
        }}
      />
      <chakra.main
        ref={main}
        id="library"
        className={
          jobsOpen
            ? 'jobs-main'
            : configuring
              ? 'setup-page'
              : section === 'Settings'
                ? 'settings-main'
                : undefined
        }
      >
        <ErrorToast
          error={error}
          onDismiss={() => setError(null)}
          onReconnect={!foundation ? reconnect : undefined}
        />
        <Box className="workspace-page" hidden={jobsOpen} inert={jobsOpen}>
          {configuring && (
            <SetupHeader
              hasLibrary={hasLibrary}
              section={section}
              busy={busy}
              jobsControl={jobsControl}
              onChoose={() => void chooseFolder()}
            />
          )}

          <NotesDraftScope key={foundation?.catalogId ?? 'connecting'}>
            <WorkspaceContent
              mapState={mapState}
              onMapState={setMapState}
              libraryLaunch={libraryLaunch}
              onReturnToMap={() => setSection('Map')}
              onShowLibrary={(query, selectedId) => {
                setLibraryLaunch((previous) => ({
                  query,
                  selectedId,
                  generation: (previous?.generation ?? 0) + 1,
                }));
                setSection('Photos');
              }}
              app={app}
              settings={settings}
              jobsControl={jobsControl}
              section={section}
              onSettings={(id) => {
                setSettingsId(id);
                setSection('Settings');
              }}
              onDefaultsSaved={() => setNavigationOverride(null)}
            />
          </NotesDraftScope>
        </Box>
        {jobsOpen && (
          <JobsPage
            status={status}
            onRefresh={refresh}
            onBack={() => {
              returnFromJobs.current = true;
              setJobsOpen(false);
            }}
          />
        )}
        <Text className="screen-reader-status" role="status">
          {notice}
        </Text>
      </chakra.main>
    </Box>
  );
}

function NotesDraftScope({ children }: { children: ReactNode }) {
  const [drafts] = useState(() => new Map<string, NotesDraft>());
  return <NotesDraftContext.Provider value={drafts}>{children}</NotesDraftContext.Provider>;
}

function NavigationToggle({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const label = collapsed ? 'Expand navigation' : 'Collapse navigation';
  return (
    <Button
      className="sidebar-toggle"
      aria-label={label}
      title={label}
      aria-expanded={!collapsed}
      aria-controls="app-sidebar"
      onClick={onToggle}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <rect x="3" y="4" width="18" height="16" rx="2" stroke="currentColor" strokeWidth="1.5" />
        <path d="M9 4v16" stroke="currentColor" strokeWidth="1.5" />
        <path
          d={collapsed ? 'm13 9 3 3-3 3' : 'm16 9-3 3 3 3'}
          stroke="currentColor"
          strokeWidth="1.5"
        />
      </svg>
      <Text as="span" className="nav-label">
        {collapsed ? 'Expand' : 'Collapse'}
      </Text>
    </Button>
  );
}

function NavigationIcon({ section }: { section: AppSection | 'Map' | 'Utilities' }) {
  const paths: Record<typeof section, string> = {
    Photos: 'M4 4h16v16H4ZM4 15l5-5 5 5 3-3 3 3',
    Folders: 'M3 7h7l2 2h9v11H3ZM3 7V4h7l2 3',
    Tags: 'M3 3h8l10 10-8 8L3 11ZM7 7h.01',
    Map: 'M3 5l6-2 6 2 6-2v16l-6 2-6-2-6 2ZM9 3v16M15 5v16',
    Utilities: 'M4 7h16M4 17h16M8 4v6M16 14v6',
    Duplicates: 'M4 4h11v13H4ZM9 8h11v13H9',
    Integrity: 'M12 3l8 3v6c0 5-8 9-8 9s-8-4-8-9V6ZM8 12l3 3 5-6',
    Storage: 'M4 4h16v16H4ZM4 11h16M12 11v9',
    Backup: 'M4 6h16v14H4ZM8 6V3h8v3M8 13h8m-4-3v6',
    Sync: 'M4 7h15m-4-4 4 4-4 4M20 17H5m4-4-4 4 4 4',
    'Structure Formater': 'M4 4h6v5H4ZM14 15h6v5h-6ZM7 9v8h7M7 13h10V5h-3',
    Settings:
      'M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8ZM9 3h6l1 3 3 1 2 5-2 5-3 1-1 3H9l-1-3-3-1-2-5 2-5 3-1Z',
  };
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d={paths[section]} stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
    </svg>
  );
}

function Sidebar({
  hasLibrary,
  version,
  section,
  onNavigate,
  collapsed,
  onToggleNavigation,
  jobsControl,
  jobsOpen,
}: {
  jobsOpen: boolean;
  jobsControl: ReactNode;
  hasLibrary: boolean;
  version: string | undefined;
  section: AppSection;
  onNavigate: (section: AppSection) => void;
  collapsed: boolean;
  onToggleNavigation: () => void;
}) {
  return (
    <chakra.aside
      id="app-sidebar"
      className="sidebar"
      aria-label="Stillharbour"
      onContextMenu={(event) => event.preventDefault()}
    >
      <Box
        height="appHeader"
        minHeight="appHeader"
        maxHeight="appHeader"
        borderBottom="1px solid {colors.border}"
        className="brand"
      >
        <Image src={logo} alt="" htmlWidth={32} htmlHeight={34} />
        <Text as="span" m="0" gap={0} lineHeight={1}>
          <Text fontSize="sm" m="0" as="span">
            Stillharbour
          </Text>
          {version && (
            <>
              {' '}
              <Text as="span" className="brand-version" aria-label="Application version">
                {version}
              </Text>
            </>
          )}
          <small>Your media library organizer.</small>
        </Text>
      </Box>
      <Box as="nav" aria-label="Main navigation" p="4">
        {hasLibrary ? (
          <>
            <Box className="nav-group" role="group" aria-label="Collection">
              {(['Photos', 'Folders', 'Tags'] as const).map((item) => (
                <Button
                  key={item}
                  aria-label={item === 'Tags' ? 'Albums / Tags' : item}
                  title={collapsed ? (item === 'Tags' ? 'Albums / Tags' : item) : undefined}
                  aria-current={!jobsOpen && section === item ? 'page' : undefined}
                  onClick={() => {
                    onNavigate(item);
                  }}
                >
                  <NavigationIcon section={item} />
                  <Text as="span" className="nav-label">
                    {item === 'Tags' ? 'Albums / Tags' : item}
                  </Text>
                </Button>
              ))}
              <Button
                aria-label="Map"
                title={collapsed ? 'Map' : undefined}
                aria-current={!jobsOpen && section === 'Map' ? 'page' : undefined}
                onClick={() => onNavigate('Map')}
              >
                <NavigationIcon section="Map" />
                <Text as="span" className="nav-label">
                  Map
                </Text>
              </Button>
            </Box>
            <Box className="nav-group" role="group" aria-label="Utilities">
              <Box className="nav-group-label">Utilities</Box>
              {(
                [
                  'Structure Formater',
                  'Duplicates',
                  'Integrity',
                  'Storage',
                  'Backup',
                  'Sync',
                ] as const
              ).map((item) => (
                <Button
                  key={item}
                  aria-label={item}
                  title={item}
                  aria-current={!jobsOpen && section === item ? 'page' : undefined}
                  onClick={() => onNavigate(item)}
                >
                  <NavigationIcon section={item} />
                  <Text as="span" className="nav-label">
                    {item}
                  </Text>
                </Button>
              ))}
            </Box>
          </>
        ) : (
          <Button
            onClick={() => onNavigate('Photos')}
            aria-current={!jobsOpen && section === 'Photos' ? 'page' : undefined}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <rect x="3" y="3" width="7" height="7" rx="1" />
              <rect x="14" y="3" width="7" height="7" rx="1" />
              <rect x="3" y="14" width="7" height="7" rx="1" />
              <rect x="14" y="14" width="7" height="7" rx="1" />
            </svg>
            <Text as="span" className="nav-label">
              Library
            </Text>
          </Button>
        )}
      </Box>
      <Box as="footer" className="sidebar-footer" py="4" px="5">
        <chakra.nav aria-label="Workspace actions">
          {jobsControl}
          {(['Settings'] as const).map((item) => (
            <Button
              key={item}
              aria-label={item}
              title={item}
              aria-current={!jobsOpen && section === item ? 'page' : undefined}
              onClick={() => onNavigate(item)}
            >
              <NavigationIcon section={item} />
              <Text as="span" className="nav-label">
                {item}
              </Text>
            </Button>
          ))}
          <NavigationToggle collapsed={collapsed} onToggle={onToggleNavigation} />
        </chakra.nav>
      </Box>
    </chakra.aside>
  );
}

function LibraryWelcome({
  hasLibrary,
  busy,
  disabled,
  onChoose,
}: {
  hasLibrary: boolean;
  busy: boolean;
  disabled: boolean;
  onChoose: () => void;
}) {
  return (
    <chakra.section className="welcome" aria-labelledby="welcome-title">
      <Box className="folder-mark">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none" aria-hidden="true">
          <path
            d="M5 12a3 3 0 0 1 3-3h9l4 4h11a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V12Z"
            stroke="currentColor"
            strokeWidth="1.5"
          />
          <path d="M5 18h30" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </Box>
      <Heading as="h2" unstyled id="welcome-title">
        {hasLibrary ? 'Add another library folder.' : 'A home for the moments you keep.'}
      </Heading>
      <Text className="welcome-description">
        Choose your photo library folder. Your originals stay in place.
      </Text>
      <Button className="primary" onClick={onChoose} disabled={disabled}>
        {busy ? 'Please wait…' : 'Choose a folder'}
      </Button>
    </chakra.section>
  );
}

function LibraryOnboarding({
  defaults,
  hasLibrary,
  selected,
  root,
  busy,
  disabled,
  onChoose,
  onSave,
  onCancel,
  onError,
}: {
  hasLibrary: boolean;
  selected: DirectorySelection | null;
  root: LibraryRoot | undefined;
  busy: boolean;
  disabled: boolean;
  onChoose: () => void;
  defaults: import('../../bindings').AppPreferences;
  onSave: (label: string, mode: RootMode, filter: ScanFilter) => Promise<void>;
  onCancel: () => void;
  onError: (error: unknown) => void;
}) {
  return (
    <Box className="workspace">
      {!root && !selected && (
        <LibraryWelcome
          hasLibrary={hasLibrary}
          busy={busy}
          disabled={disabled}
          onChoose={onChoose}
        />
      )}{' '}
      {(selected || root) && (
        <RootSetup
          key={root?.id ?? selected?.id}
          defaults={defaults}
          selected={selected ?? undefined}
          root={root}
          busy={busy}
          onSave={onSave}
          onCancel={onCancel}
          onError={onError}
        />
      )}
    </Box>
  );
}

function SetupHeader({
  jobsControl,
  hasLibrary,
  section,
  busy,
  onChoose,
}: {
  hasLibrary: boolean;
  section: AppSection;
  jobsControl: ReactNode;
  busy: boolean;
  onChoose: () => void;
}) {
  return (
    <PageHeader>
      <chakra.header className="toolbar">
        <Heading as="h1" unstyled>
          {hasLibrary ? (section === 'Tags' ? 'Albums / Tags' : section) : 'Your library'}
        </Heading>
        <Box className="header-actions">
          {hasLibrary && (
            <Button onClick={onChoose} disabled={busy}>
              Add folder
            </Button>
          )}
          {jobsControl}
        </Box>
      </chakra.header>
    </PageHeader>
  );
}

function WorkspaceContent({
  mapState,
  onMapState,
  libraryLaunch,
  onShowLibrary,
  onReturnToMap,
  jobsControl,
  app,
  settings,
  section,
  onDefaultsSaved,
  onSettings,
}: {
  mapState: MapBrowseState;
  onMapState: (state: MapBrowseState) => void;
  libraryLaunch: LibraryLaunch | null;
  onShowLibrary: (query: AssetQuery, selectedId: string | null) => void;
  onReturnToMap: () => void;
  jobsControl: ReactNode;
  app: ReturnType<typeof useFoundation>;
  settings: ReturnType<typeof usePreferences>;
  section: AppSection;
  onDefaultsSaved: () => void;
  onSettings: (id: string) => void;
}) {
  const [activeAlbum, setActiveAlbum] = useState<AlbumSummary | null>(null);
  const [addingAlbum, setAddingAlbum] = useState<CatalogCollection | null>(null);
  const preferences = settings.preferences;
  if (!preferences)
    return (
      <Box className="settings-loading" role="status">
        {settings.error ? (
          <>
            <Text role="alert">{settings.error}</Text>
            <Button onClick={settings.retry}>Retry settings</Button>
          </>
        ) : (
          <Text>Loading settings…</Text>
        )}
      </Box>
    );
  const {
    status,
    selected,
    busy,
    loading,
    foundation,
    showError,
    save,
    settingsId,
    setSelected,
    setSettingsId,
    chooseFolder,
    refresh,
    rescan,
  } = app;
  const hasLibrary = Boolean(status?.roots.length);
  if (selected || (!hasLibrary && section !== 'Settings'))
    return (
      <LibraryOnboarding
        defaults={preferences}
        hasLibrary={hasLibrary}
        selected={selected}
        root={undefined}
        busy={busy}
        disabled={loading || !foundation || busy}
        onChoose={() => void chooseFolder()}
        onSave={save}
        onCancel={() => {
          setSelected(null);
          setSettingsId(null);
        }}
        onError={showError}
      />
    );
  if (section === 'Settings')
    return (
      <SettingsScreen
        catalogId={foundation?.catalogId ?? null}
        onSaveImportDestination={async (rootId) => {
          if (!foundation) throw new Error('Open a catalog before saving its import destination.');
          const value = await libraryData(
            {
              kind: 'organization_import_destination_set',
              catalog_id: foundation.catalogId,
              root_id: rootId,
            },
            'organization_import_destination',
          );
          await refresh();
          return value;
        }}
        jobsControl={jobsControl}
        preferences={preferences}
        status={status}
        onSave={async (value) => {
          const saved = await settings.save(value);
          onDefaultsSaved();
          return saved;
        }}
        onAddFolder={() => void chooseFolder()}
        onRelink={chooseFolder}
        initialRootId={settingsId}
        onRescan={rescan}
        onMaintenance={async () => {
          await refresh();
          settings.retry();
        }}
        locationBusy={busy}
        onError={showError}
      />
    );
  if (section === 'Structure Formater')
    return status ? (
      <StructureFormatter
        key={foundation?.catalogId ?? 'catalog'}
        status={status}
        catalogId={foundation?.catalogId ?? null}
      />
    ) : null;
  if (section === 'Sync')
    return status ? (
      <SyncScreen
        key={foundation?.catalogId ?? 'catalog'}
        status={status}
        navigationControl={null}
        jobsControl={jobsControl}
      />
    ) : null;
  if (section === 'Backup')
    return status ? (
      <BackupScreen
        navigationControl={null}
        key={foundation?.catalogId ?? 'catalog'}
        status={status}
        catalogId={foundation?.catalogId ?? null}
        jobsControl={jobsControl}
      />
    ) : null;
  if (section === 'Map')
    return status ? (
      <MapScreen
        status={status}
        state={mapState}
        onState={onMapState}
        onShowLibrary={onShowLibrary}
        jobsControl={jobsControl}
      />
    ) : null;
  if ((section === 'Duplicates' || section === 'Integrity' || section === 'Storage') && status)
    return (
      <InspectionScreen
        navigationControl={null}
        key={`${foundation?.catalogId ?? 'catalog'}:${section}`}
        section={section}
        status={status}
        jobsControl={jobsControl}
      />
    );
  if (section === 'Duplicates' || section === 'Integrity' || section === 'Storage') return null;
  if (section === 'Folders' && status)
    return (
      <FolderExplorer
        jobsControl={jobsControl}
        status={status}
        onError={showError}
        onRefresh={refresh}
      />
    );
  if (section === 'Tags' && status && !activeAlbum)
    return (
      <AlbumsPage
        onBrowse={(query) => onShowLibrary(query, null)}
        status={status}
        jobsControl={jobsControl}
        onOpen={setActiveAlbum}
        onRefresh={refresh}
      />
    );
  const album = section === 'Tags' ? activeAlbum : null;
  return (
    status && (
      <>
        <LibraryScreen
          key={album?.id ?? libraryLaunch?.generation ?? 'library'}
          initialQuery={album?.query ?? libraryLaunch?.query}
          albumControls={
            album ? (
              <Box className="album-detail-bar">
                <Button onClick={() => setActiveAlbum(null)}>← All albums</Button>
                <Box>
                  <Heading as="h2" unstyled>
                    {album.name}
                  </Heading>
                  <Text as="span">
                    {album.fromTags
                      ? 'From tags · Collected automatically from photo tags'
                      : 'Custom album'}
                  </Text>
                </Box>
                {!album.fromTags && (
                  <Button
                    onClick={() => {
                      void metadata({ kind: 'collections' }, 'collections')
                        .then((items) => {
                          const current = items.find((c) => c.id === album.id && !c.archived);
                          if (current) setAddingAlbum(current);
                          else
                            showError(
                              new Error('This album is no longer available. Return to all albums.'),
                            );
                        })
                        .catch(showError);
                    }}
                  >
                    Add photos
                  </Button>
                )}
              </Box>
            ) : undefined
          }
          initialSelectedId={libraryLaunch?.selectedId ?? null}
          onReturnToMap={libraryLaunch ? onReturnToMap : undefined}
          catalogId={foundation?.catalogId ?? null}
          jobsControl={jobsControl}
          defaults={preferences}
          status={status}
          section={section}
          onError={showError}
          onRefresh={refresh}
          onSettings={onSettings}
          onRelink={chooseFolder}
        />
        {addingAlbum && album && (
          <AlbumEditor
            status={status}
            album={addingAlbum}
            onClose={() => setAddingAlbum(null)}
            onSaved={() => {
              setAddingAlbum(null);
              void refresh();
            }}
          />
        )}
      </>
    )
  );
}
