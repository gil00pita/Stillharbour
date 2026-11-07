import { Heading, Text, chakra } from '@chakra-ui/react';
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
    <chakra.section className="settings-section" aria-labelledby="folder-defaults-title">
      <Heading as="h2" unstyled id="folder-defaults-title">
        New folder defaults
      </Heading>
      <Text className="help">
        Used for folders you add next. Existing folders keep their saved mode and scan filters.
      </Text>
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
    </chakra.section>
  );
}
