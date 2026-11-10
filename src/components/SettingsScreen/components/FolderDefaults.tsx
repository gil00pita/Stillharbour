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
    <chakra.section
      className="settings-section"
      css={{
        padding: '32px 0',
        borderBottom: '1px solid {colors.border}',
        '& h2': {
          fontSize: '16px',
          fontWeight: '600',
          letterSpacing: '-0.01em',
          margin: '0 0 8px',
        },
        '& h3': { fontSize: '14px', margin: '0 0 8px' },
        '& > .help': { maxWidth: '72ch' },
        '& .view-switch': { margin: '16px 0', flexWrap: 'wrap', gap: '8px' },
        '& .view-switch > button': { fontSize: '13px !important' },
        '& .thumbnail-control': { fontSize: '13px' },
        '& .scan-options': { marginTop: '20px' },
      }}
      aria-labelledby="folder-defaults-title"
    >
      <Heading as="h2" unstyled id="folder-defaults-title">
        New folder defaults
      </Heading>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
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
