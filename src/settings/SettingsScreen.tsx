import type { ReactNode, Ref } from 'react';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type {
  AppPreferences,
  LibraryStatus,
  MaintenanceTarget,
  MaintenanceResult,
} from '../bindings';
import { errorMessage } from '../ipc';
import { Switch } from '../components/Switch';
import { ScanOptions } from '../library/ScanOptions';
import { scanFilterFromForm } from '../library/scanForm';
import { BrowserControls } from '../library/BrowserControls';
import { LibraryMaintenance } from './LibraryMaintenance';
import { ScanSettingsGroup } from './ScanSettingsGroup';
type SettingsScreenProps = {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  preferences: AppPreferences;
  status: LibraryStatus | null;
  onSave: (value: AppPreferences) => Promise<AppPreferences>;
  onAddFolder: () => void;
  onRelink: (id: string) => Promise<void>;
  initialRootId: string | null;
  onRescan: (
    id: string,
    mode: AppPreferences['rootMode'],
    filter: AppPreferences['scanFilter'],
  ) => Promise<void>;
  locationBusy: boolean;
  onMaintenance: (result: MaintenanceResult) => Promise<void>;
  onError: (error: unknown) => void;
};

export function SettingsScreen({
  jobsControl,
  navigationControl,
  preferences,
  status,
  onSave,
  onAddFolder,
  onRelink,
  initialRootId,
  onRescan,
  locationBusy,
  onMaintenance,
  onError,
}: SettingsScreenProps) {
  const [maintenanceTarget, setMaintenanceTarget] = useState<MaintenanceTarget | null>(null);
  const controlsBusy = locationBusy || Boolean(maintenanceTarget);
  const maintenanceOpener = useRef<HTMLButtonElement | null>(null);
  const [draft, setDraft] = useState(preferences);
  const [scanRevision, resetScan] = useState(0);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    if (!initialRootId) heading.current?.focus();
  }, [initialRootId]);
  function update(patch: Partial<AppPreferences>) {
    setDraft((value) => ({ ...value, ...patch }));
    setSaved(false);
  }
  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!event.currentTarget.reportValidity()) return;
    const value = {
      ...draft,
      scanFilter: scanFilterFromForm(draft.scanFilter, event.currentTarget),
    };
    setBusy(true);
    setError(null);
    setSaved(false);
    try {
      setDraft(await onSave(value));
      setSaved(true);
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <div className="settings-page">
      <SettingsHeading
        heading={heading}
        jobsControl={jobsControl}
        navigationControl={navigationControl}
      />
      <section className="settings-section" aria-labelledby="library-locations-title">
        <div className="settings-section-heading">
          <div>
            <h2 id="library-locations-title">Library locations</h2>
            <p className="help">Photo folders indexed in place. Your photos stay where they are.</p>
          </div>
          <button disabled={controlsBusy || busy || !status} onClick={onAddFolder}>
            Add photo folder
          </button>
        </div>
        {!status ? (
          <p role="status">Connecting to your libraries…</p>
        ) : (
          <>
            {!status.roots.length && <p className="help">No photo folders added yet.</p>}
            <ul className="settings-libraries">
              {status.roots.map((root) => (
                <li key={root.id}>
                  <div>
                    <h3>{root.label}</h3>
                    <p className="path">
                      <bdi>{root.displayPath}</bdi>
                    </p>
                    <p className="help">
                      {root.availability.replaceAll('_', ' ')} · {root.fileCount.toLocaleString()}{' '}
                      indexed · {root.mode === 'managed' ? 'Managed' : 'Read only'}
                    </p>
                  </div>
                  <div className="settings-library-actions">
                    <button
                      disabled={controlsBusy || busy || root.availability === 'scanning'}
                      onClick={() => void onRelink(root.id)}
                      aria-label={`Change location for ${root.label}`}
                    >
                      Change location
                    </button>
                    <button
                      type="button"
                      className="destructive"
                      disabled={
                        controlsBusy ||
                        busy ||
                        Boolean(maintenanceTarget) ||
                        root.availability === 'scanning'
                      }
                      onClick={(event) => {
                        maintenanceOpener.current = event.currentTarget;
                        setMaintenanceTarget({ kind: 'library', root_id: root.id });
                      }}
                      aria-label={`Remove library ${root.label}`}
                    >
                      Remove library
                    </button>
                  </div>
                </li>
              ))}
            </ul>
            <p className="help">
              Change location reconnects the same folder after a move. To index a different folder,
              use Add photo folder. A changed location is checked before reconciliation starts.
            </p>
          </>
        )}
      </section>
      <section className="settings-section" aria-labelledby="preview-cache-title">
        <h2 id="preview-cache-title">Preview cache</h2>
        <p className="help">
          Clear Stillharbour’s indexed cached previews for all libraries. Original photos and
          catalog notes stay unchanged.
        </p>
        <button
          type="button"
          disabled={
            controlsBusy ||
            busy ||
            Boolean(maintenanceTarget) ||
            !status ||
            status.jobs.some((job) => ['queued', 'running', 'cancel_requested'].includes(job.state))
          }
          onClick={(event) => {
            maintenanceOpener.current = event.currentTarget;
            setMaintenanceTarget({ kind: 'preview_cache' });
          }}
        >
          Clear preview cache
        </button>
      </section>
      {maintenanceTarget && (
        <LibraryMaintenance
          key={JSON.stringify(maintenanceTarget)}
          target={maintenanceTarget}
          onClose={() => {
            setMaintenanceTarget(null);
            // Focus return needs the DOM commit, not a painted frame: native
            // WebViews can suspend animation frames while occluded.
            setTimeout(() => {
              if (maintenanceOpener.current?.isConnected) maintenanceOpener.current.focus();
              else heading.current?.focus();
            }, 0);
          }}
          onComplete={async (result) => {
            if (result.removedRootId)
              setDraft((value) =>
                value.defaultRootId === result.removedRootId
                  ? { ...value, defaultRootId: null }
                  : value,
              );
            await onMaintenance(result);
          }}
        />
      )}
      <ScanSettingsGroup
        roots={status?.roots ?? []}
        initialRootId={initialRootId}
        busy={busy || controlsBusy}
        onRescan={onRescan}
        onError={onError}
      />
      <form
        className="settings-defaults-form"
        aria-label="Application defaults"
        onInput={() => setSaved(false)}
        onSubmit={(event) => void save(event)}
      >
        <fieldset disabled={busy || controlsBusy} className="settings-fields">
          <section className="settings-section" aria-labelledby="browsing-defaults-title">
            <h2 id="browsing-defaults-title">Browsing defaults</h2>
            <p className="help">
              Applied when you open Photos. Changes here do not alter your catalog notes or scan
              existing folders.
            </p>
            <div className="form-grid">
              <label>
                Default library
                <select
                  aria-label="Default library"
                  value={draft.defaultRootId ?? ''}
                  onChange={(event) => update({ defaultRootId: event.target.value || null })}
                >
                  <option value="">All libraries</option>
                  {draft.defaultRootId &&
                    !status?.roots.some((root) => root.id === draft.defaultRootId) && (
                      <option value={draft.defaultRootId}>Unavailable saved library</option>
                    )}
                  {status?.roots.map((root) => (
                    <option key={root.id} value={root.id}>
                      {root.label}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Default sort
                <select
                  aria-label="Default sort"
                  value={draft.sort}
                  onChange={(event) =>
                    update({ sort: event.target.value as AppPreferences['sort'] })
                  }
                >
                  <option value="capture">Date · newest first</option>
                  <option value="added">Recently added</option>
                  <option value="modified">File modified · newest first</option>
                  <option value="name">Filename</option>
                  <option value="size">Size · largest first</option>
                </select>
              </label>
            </div>
            <BrowserControls
              mode={draft.galleryView}
              size={draft.thumbnailSize}
              fit={draft.gridFit}
              onMode={(galleryView) => update({ galleryView })}
              onSize={(thumbnailSize) => update({ thumbnailSize })}
              onFit={(gridFit) => update({ gridFit })}
            />
            <Switch
              label="Start with navigation collapsed"
              checked={draft.navigationCollapsed}
              onChange={(navigationCollapsed) => update({ navigationCollapsed })}
            />
          </section>
          <section className="settings-section" aria-labelledby="folder-defaults-title">
            <h2 id="folder-defaults-title">New folder defaults</h2>
            <p className="help">
              Used for folders you add next. Existing folders keep their saved mode and scan
              filters.
            </p>
            <Switch
              label="Read only by default"
              checked={draft.rootMode === 'referenced'}
              onChange={(readOnly) => update({ rootMode: readOnly ? 'referenced' : 'managed' })}
            />
            <ScanOptions
              key={scanRevision}
              filter={draft.scanFilter}
              disabled={busy || controlsBusy}
              onChange={(scanFilter) => update({ scanFilter })}
              onError={onError}
            />
          </section>
          <div className="settings-save">
            <button
              type="button"
              onClick={() => {
                setDraft(preferences);
                resetScan((value) => value + 1);
                setSaved(false);
                setError(null);
              }}
            >
              Discard changes
            </button>
            <button type="submit" className="primary">
              {busy ? 'Saving…' : 'Save defaults'}
            </button>
          </div>
        </fieldset>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {saved && (
          <p role="status" className="settings-saved">
            Defaults saved. Existing photo folders are unchanged.
          </p>
        )}
      </form>
    </div>
  );
}

function SettingsHeading({
  heading,
  jobsControl,
  navigationControl,
}: {
  heading: Ref<HTMLHeadingElement>;
  jobsControl: ReactNode;
  navigationControl: ReactNode;
}) {
  return (
    <header className="settings-heading">
      <div className="settings-title-row">
        {navigationControl}
        <h1 ref={heading} tabIndex={-1}>
          Settings
        </h1>
        {jobsControl}
      </div>
      <p>Choose your photo folders and the defaults for your workspace.</p>
    </header>
  );
}
