import type { AppPreferences } from '../../../bindings';
import { Switch } from '@/components/Switch';
import { ScanOptions } from '@/components/ScanOptions';

export function FolderDefaults({
  draft,
  busy,
  scanRevision,
  onChange: update,
  onError,
}: {
  draft: AppPreferences;
  busy: boolean;
  scanRevision: number;
  onChange: (patch: Partial<AppPreferences>) => void;
  onError: (error: unknown) => void;
}) {
  return (
    <section className="settings-section" aria-labelledby="folder-defaults-title">
      <h2 id="folder-defaults-title">New folder defaults</h2>
      <p className="help">
        Used for folders you add next. Existing folders keep their saved mode and scan filters.
      </p>
      <Switch
        label="Read only by default"
        checked={draft.rootMode === 'referenced'}
        onChange={(readOnly) => update({ rootMode: readOnly ? 'referenced' : 'managed' })}
      />
      <ScanOptions
        key={scanRevision}
        filter={draft.scanFilter}
        disabled={busy}
        onChange={(scanFilter) => update({ scanFilter })}
        onError={onError}
      />
    </section>
  );
}
