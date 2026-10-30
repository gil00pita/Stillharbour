import { useState, type ReactNode } from 'react';
import { useFoundation } from './useFoundation';
import { usePreferences } from './usePreferences';
import { SettingsScreen } from './settings/SettingsScreen';
import type { DirectorySelection, LibraryRoot, RootMode, ScanFilter } from './bindings';
import { JobsDropdown } from './library/JobsDropdown';
import { LibraryScreen, type Section } from './library/LibraryScreen';
import { FolderExplorer } from './library/FolderExplorer';
import { RootSetup } from './library/RootSetup';
import logo from '../assets/branding/stillharbour-logo.svg';

type AppSection = Section | 'Settings';

function FolderIcon() {
  return (
    <svg width="40" height="40" viewBox="0 0 40 40" fill="none" aria-hidden="true">
      <path
        d="M5 12a3 3 0 0 1 3-3h9l4 4h11a3 3 0 0 1 3 3v14a3 3 0 0 1-3 3H8a3 3 0 0 1-3-3V12Z"
        stroke="currentColor"
        strokeWidth="1.5"
      />
      <path d="M5 18h30" stroke="currentColor" strokeWidth="1.5" />
    </svg>
  );
}

export default function App() {
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
  const [navigationOverride, setNavigationOverride] = useState<boolean | null>(null);
  const collapsed = navigationOverride ?? preferences?.navigationCollapsed ?? false;
  const hasLibrary = Boolean(status?.roots.length);
  const jobsControl = <JobsDropdown status={status} onRefresh={refresh} />;
  const navigationControl = (
    <NavigationToggle collapsed={collapsed} onToggle={() => setNavigationOverride(!collapsed)} />
  );
  const configuring = Boolean(selected) || (!hasLibrary && section !== 'Settings');
  return (
    <div
      className={`app-shell ${hasLibrary ? 'has-library' : ''} ${collapsed ? 'navigation-collapsed' : ''}`}
    >
      <Sidebar
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
        <WorkspaceContent
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
        <p className="screen-reader-status" role="status">
          {notice}
        </p>
      </main>
    </div>
  );
}

function NavigationToggle({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const label = collapsed ? 'Expand navigation' : 'Collapse navigation';
  return (
    <button
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
    </button>
  );
}

function NavigationIcon({ section }: { section: AppSection | 'Map' | 'Utilities' }) {
  const paths: Record<typeof section, string> = {
    Photos: 'M4 4h16v16H4ZM4 15l5-5 5 5 3-3 3 3',
    Folders: 'M3 7h7l2 2h9v11H3ZM3 7V4h7l2 3',
    Tags: 'M3 3h8l10 10-8 8L3 11ZM7 7h.01',
    Map: 'M3 5l6-2 6 2 6-2v16l-6 2-6-2-6 2ZM9 3v16M15 5v16',
    Utilities: 'M4 7h16M4 17h16M8 4v6M16 14v6',
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
}: {
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
              <button
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
              </button>
            ))}
            <button disabled title="Map browsing is planned for a later milestone">
              <NavigationIcon section="Map" />
              <span className="nav-label">Map</span>
              <small>Later</small>
            </button>
            <button disabled title="File operations remain unavailable in this read-only release">
              <NavigationIcon section="Utilities" />
              <span className="nav-label">Utilities</span>
              <small>Later</small>
            </button>
          </>
        ) : (
          <button
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
          </button>
        )}
      </nav>
      <footer className="sidebar-footer">
        <nav aria-label="Workspace actions">
          {(['Settings'] as const).map((item) => (
            <button
              key={item}
              aria-label={item}
              title={collapsed ? item : undefined}
              aria-current={section === item ? 'page' : undefined}
              onClick={() => onNavigate(item)}
            >
              <NavigationIcon section={item} />
              <span className="nav-label">{item}</span>
            </button>
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
        <FolderIcon />
      </div>
      <h2 id="welcome-title">
        {hasLibrary
          ? 'Add another library folder.'
          : 'Start by selecting your Photo library folder.'}
      </h2>
      <button className="primary" onClick={onChoose} disabled={disabled}>
        {busy ? 'Please wait…' : 'Choose a folder'}
      </button>
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
        <button onClick={onReconnect}>Reconnect</button>
      ) : (
        <button onClick={onDismiss}>Dismiss</button>
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
          <button onClick={onChoose} disabled={busy}>
            Add folder
          </button>
        )}
        {jobsControl}
      </div>
    </header>
  );
}

function WorkspaceContent({
  jobsControl,
  navigationControl,
  app,
  settings,
  section,
  onDefaultsSaved,
  onSettings,
}: {
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
            <button onClick={settings.retry}>Retry settings</button>
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
