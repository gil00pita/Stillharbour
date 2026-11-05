import { Button } from './components/ui/controls';
import { useState, type ReactNode } from 'react';
import { useFoundation } from './useFoundation';
import { usePreferences } from './usePreferences';
import { SettingsScreen } from './settings/SettingsScreen';
import type { AssetQuery, DirectorySelection, LibraryRoot, RootMode, ScanFilter } from './bindings';
import { MapScreen } from './map/MapScreen';
import { initialMapState, type MapBrowseState } from './map/state';
import { JobsDropdown } from './library/JobsDropdown';
import { LibraryScreen, type Section } from './library/LibraryScreen';
import { FolderExplorer } from './library/FolderExplorer';
import { InspectionScreen, type InspectionSection } from './inspection/InspectionScreen';
import { RootSetup } from './library/RootSetup';
import logo from '../assets/branding/stillharbour-logo.svg';
import { libraryData } from './ipc';
import { BackupScreen } from './backup/BackupScreen';
import { useCatalogSwitchState } from './backup/catalogSwitchState';
import { CatalogSwitchStopped } from './backup/CatalogSwitchStopped';
import { NotesDraftContext, type NotesDraft } from './metadata/notesDrafts';

type AppSection = Section | 'Settings' | InspectionSection | 'Backup & Sync' | 'Map';
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
  const [mapState, setMapState] = useState(initialMapState);
  const [libraryLaunch, setLibraryLaunch] = useState<LibraryLaunch | null>(null);
  const [navigationOverride, setNavigationOverride] = useState<boolean | null>(null);
  const collapsed = navigationOverride ?? preferences?.navigationCollapsed ?? false;
  const hasLibrary = Boolean(status?.roots.length);
  const jobsControl = null;
  const navigationControl = (
    <NavigationToggle collapsed={collapsed} onToggle={() => setNavigationOverride(!collapsed)} />
  );
  const configuring = Boolean(selected) || (!hasLibrary && section !== 'Settings');
  return (
    <div
      className={`app-shell ${hasLibrary ? 'has-library' : ''} ${collapsed ? 'navigation-collapsed' : ''}`}
    >
      <Sidebar
        jobsControl={<JobsDropdown status={status} onRefresh={refresh} sidebar />}
        hasLibrary={hasLibrary}
        version={foundation?.version}
        section={section}
        collapsed={collapsed}
        onNavigate={(item) => {
          setSection(item);
          setSelected(null);
          setSettingsId(null);
        }}
      />
      <main
        id="library"
        className={
          configuring ? 'setup-page' : section === 'Settings' ? 'settings-main' : undefined
        }
      >
        {configuring && (
          <SetupHeader
            hasLibrary={hasLibrary}
            section={section}
            busy={busy}
            jobsControl={jobsControl}
            navigationControl={navigationControl}
            onChoose={() => void chooseFolder()}
          />
        )}
        <ConnectionNotice
          error={error}
          canReconnect={!foundation}
          onDismiss={() => setError(null)}
          onReconnect={reconnect}
        />
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
            navigationControl={navigationControl}
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
        <p className="screen-reader-status" role="status">
          {notice}
        </p>
      </main>
    </div>
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
        <path
          d={collapsed ? 'm9 6 6 6-6 6' : 'm15 6-6 6 6 6'}
          stroke="currentColor"
          strokeWidth="1.5"
        />
      </svg>
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
    'Backup & Sync': 'M4 6h16v14H4ZM8 6V3h8v3M8 13h8m-4-3v6',
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
  jobsControl,
}: {
  jobsControl: ReactNode;
  hasLibrary: boolean;
  version: string | undefined;
  section: AppSection;
  onNavigate: (section: AppSection) => void;
  collapsed: boolean;
}) {
  return (
    <aside id="app-sidebar" className="sidebar" aria-label="Stillharbour">
      <div className="brand">
        <img src={logo} alt="" width="32" height="34" />
        <span>
          Stillharbour<small>Photo &amp; Video Library</small>
        </span>
      </div>
      <nav aria-label="Main navigation">
        {hasLibrary ? (
          <>
            {(['Photos', 'Folders', 'Tags'] as const).map((item) => (
              <Button
                key={item}
                aria-label={item === 'Tags' ? 'Albums / Tags' : item}
                title={collapsed ? (item === 'Tags' ? 'Albums / Tags' : item) : undefined}
                aria-current={section === item ? 'page' : undefined}
                onClick={() => {
                  onNavigate(item);
                }}
              >
                <NavigationIcon section={item} />
                <span className="nav-label">{item === 'Tags' ? 'Albums / Tags' : item}</span>
              </Button>
            ))}
            <Button
              aria-label="Map"
              title={collapsed ? 'Map' : undefined}
              aria-current={section === 'Map' ? 'page' : undefined}
              onClick={() => onNavigate('Map')}
            >
              <NavigationIcon section="Map" />
              <span className="nav-label">Map</span>
            </Button>
            <div className="nav-group-label">Utilities</div>
            {(['Duplicates', 'Integrity', 'Storage', 'Backup & Sync'] as const).map((item) => (
              <Button
                key={item}
                aria-label={item}
                title={collapsed ? item : undefined}
                aria-current={section === item ? 'page' : undefined}
                onClick={() => onNavigate(item)}
              >
                <NavigationIcon section={item} />
                <span className="nav-label">{item}</span>
              </Button>
            ))}
          </>
        ) : (
          <Button
            onClick={() => onNavigate('Photos')}
            aria-current={section === 'Photos' ? 'page' : undefined}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <rect x="3" y="3" width="7" height="7" rx="1" />
              <rect x="14" y="3" width="7" height="7" rx="1" />
              <rect x="3" y="14" width="7" height="7" rx="1" />
              <rect x="14" y="14" width="7" height="7" rx="1" />
            </svg>
            <span className="nav-label">Library</span>
          </Button>
        )}
      </nav>
      <footer className="sidebar-footer">
        <nav aria-label="Workspace actions">
          {jobsControl}
          {(['Settings'] as const).map((item) => (
            <Button
              key={item}
              aria-label={item}
              title={collapsed ? item : undefined}
              aria-current={section === item ? 'page' : undefined}
              onClick={() => onNavigate(item)}
            >
              <NavigationIcon section={item} />
              <span className="nav-label">{item}</span>
            </Button>
          ))}
        </nav>
        {version && (
          <p className="sidebar-version" aria-label="Application version">
            Version {version}
          </p>
        )}
      </footer>
    </aside>
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
    <section className="welcome" aria-labelledby="welcome-title">
      <div className="folder-mark">
        <svg width="40" height="40" viewBox="0 0 40 40" fill="none" aria-hidden="true">
          <path
            d="M5 12a3 3 0 0 1 3-3h9l4 4h11a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V12Z"
            stroke="currentColor"
            strokeWidth="1.5"
          />
          <path d="M5 18h30" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      </div>
      <h2 id="welcome-title">
        {hasLibrary
          ? 'Add another library folder.'
          : 'Start by selecting your Photo library folder.'}
      </h2>
      <Button className="primary" onClick={onChoose} disabled={disabled}>
        {busy ? 'Please wait…' : 'Choose a folder'}
      </Button>
    </section>
  );
}

function ConnectionNotice({
  error,
  canReconnect,
  onDismiss,
  onReconnect,
}: {
  error: string | null;
  canReconnect: boolean;
  onDismiss: () => void;
  onReconnect: () => void;
}) {
  if (!error) return null;
  return (
    <div className="error global-error" role="alert">
      <p>{error}</p>
      {canReconnect ? (
        <Button onClick={onReconnect}>Reconnect</Button>
      ) : (
        <Button onClick={onDismiss}>Dismiss</Button>
      )}
    </div>
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
  defaults: import('./bindings').AppPreferences;
  onSave: (label: string, mode: RootMode, filter: ScanFilter) => Promise<void>;
  onCancel: () => void;
  onError: (error: unknown) => void;
}) {
  return (
    <div className="workspace">
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
    </div>
  );
}

function SetupHeader({
  jobsControl,
  navigationControl,
  hasLibrary,
  section,
  busy,
  onChoose,
}: {
  hasLibrary: boolean;
  section: AppSection;
  jobsControl: ReactNode;
  navigationControl: ReactNode;
  busy: boolean;
  onChoose: () => void;
}) {
  return (
    <header className="toolbar">
      {navigationControl}
      <h1>{hasLibrary ? (section === 'Tags' ? 'Albums / Tags' : section) : 'Your library'}</h1>
      <div className="header-actions">
        {hasLibrary && (
          <Button onClick={onChoose} disabled={busy}>
            Add folder
          </Button>
        )}
        {jobsControl}
      </div>
    </header>
  );
}

function WorkspaceContent({
  mapState,
  onMapState,
  libraryLaunch,
  onShowLibrary,
  onReturnToMap,
  jobsControl,
  navigationControl,
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
  navigationControl: ReactNode;
  app: ReturnType<typeof useFoundation>;
  settings: ReturnType<typeof usePreferences>;
  section: AppSection;
  onDefaultsSaved: () => void;
  onSettings: (id: string) => void;
}) {
  const preferences = settings.preferences;
  if (!preferences)
    return (
      <div className="settings-loading" role="status">
        {settings.error ? (
          <>
            <p role="alert">{settings.error}</p>
            <Button onClick={settings.retry}>Retry settings</Button>
          </>
        ) : (
          <p>Loading settings…</p>
        )}
      </div>
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
        navigationControl={navigationControl}
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
  if (section === 'Backup & Sync')
    return status ? (
      <BackupScreen
        key={foundation?.catalogId ?? 'catalog'}
        status={status}
        catalogId={foundation?.catalogId ?? null}
        navigationControl={navigationControl}
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
        navigationControl={navigationControl}
        jobsControl={jobsControl}
      />
    ) : null;
  if ((section === 'Duplicates' || section === 'Integrity' || section === 'Storage') && status)
    return (
      <InspectionScreen
        key={`${foundation?.catalogId ?? 'catalog'}:${section}`}
        section={section}
        status={status}
        navigationControl={navigationControl}
        jobsControl={jobsControl}
      />
    );
  if (section === 'Duplicates' || section === 'Integrity' || section === 'Storage') return null;
  if (section === 'Folders' && status)
    return (
      <FolderExplorer
        jobsControl={jobsControl}
        navigationControl={navigationControl}
        status={status}
        onError={showError}
        onRefresh={refresh}
      />
    );
  return (
    status && (
      <LibraryScreen
        key={libraryLaunch?.generation ?? 'library'}
        initialQuery={libraryLaunch?.query}
        initialSelectedId={libraryLaunch?.selectedId ?? null}
        onReturnToMap={libraryLaunch ? onReturnToMap : undefined}
        catalogId={foundation?.catalogId ?? null}
        jobsControl={jobsControl}
        navigationControl={navigationControl}
        defaults={preferences}
        status={status}
        section={section}
        onError={showError}
        onRefresh={refresh}
        onSettings={onSettings}
        onRelink={chooseFolder}
      />
    )
  );
}
