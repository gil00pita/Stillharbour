import { Box, Text, chakra } from '@chakra-ui/react';
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
    <Box
      className="settings-page"
      css={{
        width: '100%',
        padding: '0',
        margin: '0',
        '@media (max-width: 760px)': { padding: '0' },
      }}
    >
      <SettingsHeading
        heading={heading}
        jobsControl={jobsControl}
        navigationControl={navigationControl}
      />
      <Box
        className="settings-content"
        css={{
          width: 'min(100%, 1040px)',
          padding: '24px 40px 56px',
          margin: '0 auto',
          '@media (max-width: 760px)': { paddingInline: '16px' },
        }}
      >
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Choose your photo folders and the defaults for your workspace.
        </Text>
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
        <chakra.form
          className="settings-defaults-form"
          aria-label="Application defaults"
          onInput={() => setSaved(false)}
          onSubmit={(event) => void save(event)}
        >
          <chakra.fieldset
            disabled={formsBusy}
            className="settings-fields"
            css={{ border: '0', padding: '0', margin: '0', minWidth: '0' }}
          >
            <BrowsingDefaults draft={draft} status={status} onChange={update} />
            <FolderDefaults
              draft={draft}
              busy={formsBusy}
              scanRevision={scanRevision}
              onChange={update}
              onError={onError}
            />
            <Box
              className="settings-save"
              css={{
                display: 'flex',
                flexWrap: 'wrap',
                alignItems: 'center',
                gap: '8px',
                justifyContent: 'flex-end',
                paddingTop: '24px',
                '& .primary': { marginTop: '0' },
              }}
            >
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
            </Box>
          </chakra.fieldset>
          {error && (
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
              {error}
            </Text>
          )}
          {saved && (
            <Text
              role="status"
              className="settings-saved"
              css={{ color: '{colors.brand.solid}', marginTop: '16px' }}
            >
              Defaults saved. Existing photo folders are unchanged.
            </Text>
          )}
        </chakra.form>
      </Box>
    </Box>
  );
}
