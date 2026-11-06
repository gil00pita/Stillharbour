import { LibraryLocations } from './components/LibraryLocations';
import { PreviewCacheSettings } from './components/PreviewCacheSettings';
import { BrowsingDefaults } from './components/BrowsingDefaults';
import { FolderDefaults } from './components/FolderDefaults';
import { SettingsHeading } from './components/SettingsHeading';
import { Button } from '@/components/Button';
import type { ReactNode } from 'react';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type {
  AppPreferences,
  LibraryStatus,
  MaintenanceTarget,
  MaintenanceResult,
} from '../../bindings';
import { errorMessage } from '../../ipc';
import { scanFilterFromForm } from '../../library/scanForm';
import { LibraryMaintenance } from '@/components/LibraryMaintenance';
import { ScanSettingsGroup } from '@/components/ScanSettingsGroup';
import { ImportDestinationSettings } from '@/components/ImportDestinationSettings';
import type { ImportDestinationPreference } from '../../bindings';
type SettingsScreenProps = {
  catalogId?: string | null;
  onSaveImportDestination?: (rootId: string | null) => Promise<ImportDestinationPreference>;
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
  catalogId = null,
  onSaveImportDestination,
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
  const formsBusy = busy || controlsBusy;
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
      <div className="settings-content">
        <p className="help">Choose your photo folders and the defaults for your workspace.</p>
        <LibraryLocations
          status={status}
          busy={formsBusy}
          onAddFolder={onAddFolder}
          onRelink={onRelink}
          onRemove={(id, opener) => {
            maintenanceOpener.current = opener;
            setMaintenanceTarget({ kind: 'library', root_id: id });
          }}
        />
        <PreviewCacheSettings
          status={status}
          busy={formsBusy}
          onReview={(opener) => {
            maintenanceOpener.current = opener;
            setMaintenanceTarget({ kind: 'preview_cache' });
          }}
        />
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
          busy={formsBusy}
          onRescan={onRescan}
          onError={onError}
        />
        <ImportDestinationSettings
          key={catalogId ?? 'unavailable'}
          catalogId={catalogId}
          preference={status?.importDestination ?? null}
          roots={status?.roots ?? []}
          onSave={onSaveImportDestination ?? null}
        />
        <form
          className="settings-defaults-form"
          aria-label="Application defaults"
          onInput={() => setSaved(false)}
          onSubmit={(event) => void save(event)}
        >
          <fieldset disabled={formsBusy} className="settings-fields">
            <BrowsingDefaults draft={draft} status={status} onChange={update} />
            <FolderDefaults
              draft={draft}
              busy={formsBusy}
              scanRevision={scanRevision}
              onChange={update}
              onError={onError}
            />
            <div className="settings-save">
              <Button
                type="button"
                onClick={() => {
                  setDraft(preferences);
                  resetScan((value) => value + 1);
                  setSaved(false);
                  setError(null);
                }}
              >
                Discard changes
              </Button>
              <Button type="submit" className="primary">
                {busy ? 'Saving…' : 'Save defaults'}
              </Button>
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
    </div>
  );
}
