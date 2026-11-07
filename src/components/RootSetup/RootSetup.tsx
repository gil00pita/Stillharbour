import { Field, Box, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useEffect, useId, useRef, useState } from 'react';
import type { DirectorySelection, LibraryRoot, RootMode, ScanFilter } from '../../bindings';
import { defaultFilter, normalizeFileExtensions } from '../../library/model';
import { Switch } from '@/components/Switch';
import { ScanOptions } from '@/components/ScanOptions';
import { scanFilterFromForm } from '../../library/scanForm';

export function RootSetup({
  selected,
  root,
  busy,
  onSave,
  onCancel,
  onError,
  defaults,
  embedded = false,
  scanRunning = false,
}: {
  selected?: DirectorySelection | undefined;
  root?: LibraryRoot | undefined;
  busy: boolean;
  onSave: (label: string, mode: RootMode, filter: ScanFilter) => Promise<void>;
  onCancel: () => void;
  onError: (error: unknown) => void;
  defaults?: Pick<import('../../bindings').AppPreferences, 'rootMode' | 'scanFilter'>;
  embedded?: boolean;
  scanRunning?: boolean;
}) {
  const [label, setLabel] = useState(root?.label ?? 'Photo library');
  const [mode, setMode] = useState<RootMode>(root?.mode ?? defaults?.rootMode ?? 'managed');
  const labelInput = useRef<HTMLInputElement>(null);
  const focusedSelection = useRef<string | undefined>(undefined);
  const selectionId = selected?.id;
  const identityAlertId = useId();
  const identityUnavailable = !root && selected?.identityAvailable === false;
  useEffect(() => {
    if (selectionId && !busy && focusedSelection.current !== selectionId) {
      labelInput.current?.focus();
      focusedSelection.current = selectionId;
    }
  }, [selectionId, busy]);
  const [filter, setFilter] = useState<ScanFilter>(() => ({
    ...(root?.filter ?? defaults?.scanFilter ?? defaultFilter),
    includeGlobs: [],
    extensions: normalizeFileExtensions(
      (root?.filter ?? defaults?.scanFilter ?? defaultFilter).extensions,
    ),
    fromDate: null,
    throughDate: null,
  }));
  return (
    <chakra.form
      className={`root-setup ${embedded ? 'inline-scan-settings' : 'selected-folder'}`}
      css={{
        '&.selected-folder': {
          display: 'flex',
          flexWrap: 'wrap',
          justifyContent: 'space-between',
          alignItems: 'flex-start',
          gap: '12px 24px',
          border: '1px solid {colors.border}',
          borderRadius: '8px',
          padding: '20px',
          marginTop: '32px',
        },
        marginTop: '0',
        '& > .help': { flexBasis: '100%' },
        '& .form-grid': { margin: '12px 0' },
        '&.inline-scan-settings': { minWidth: '0', '& .folder-actions': { marginTop: '20px' } },
      }}
      onSubmit={(event) => {
        event.preventDefault();
        if (!busy && !scanRunning && !identityUnavailable && event.currentTarget.reportValidity())
          void onSave(label, mode, scanFilterFromForm(filter, event.currentTarget));
      }}
    >
      <Box className="folder-information" css={{ minWidth: '0', flex: '1' }}>
        {!embedded && (
          <Heading as="h3" unstyled>
            {root ? 'Scan settings' : 'Selected folder'}
          </Heading>
        )}
        <Text
          className="path"
          css={{
            fontSize: '13px',
            lineHeight: '1.6',
            overflowWrap: 'anywhere',
            whiteSpace: 'pre-wrap',
            margin: '8px 0 0',
            color: '{colors.fg.muted}',
          }}
        >
          <bdi>{selected?.displayPath ?? root?.displayPath}</bdi>
        </Text>
      </Box>
      {!root && (
        <Text
          as="span"
          className="selection-state"
          css={{ color: '{colors.brand.solid}', fontSize: '12px' }}
        >
          Not scanned
        </Text>
      )}
      <Box
        className="form-grid"
        css={{
          display: 'grid',
          gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
          gap: '16px',
          width: '100%',
          '@media (max-width: 600px)': { gridTemplateColumns: '1fr' },
        }}
      >
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Library name</span>
            </Field.Label>
            <Input
              ref={labelInput}
              readOnly={Boolean(root)}
              disabled={busy}
              value={label}
              maxLength={128}
              required
              onChange={(event) => setLabel(event.target.value)}
            />
          </label>
        </Field.Root>
        <Box
          className="mode-field"
          css={{ display: 'flex', flexDirection: 'column', gap: '8px', minWidth: '0' }}
        >
          <Text
            as="span"
            className="field-label"
            css={{ fontSize: '12px', color: '{colors.fg.secondary}' }}
          >
            Folder mode · {mode === 'managed' ? 'Managed' : 'Read only'}
          </Text>
          <Switch
            label="Read only"
            checked={mode === 'referenced'}
            disabled={busy}
            onChange={(readOnly) => setMode(readOnly ? 'referenced' : 'managed')}
          />
        </Box>
      </Box>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        {mode === 'referenced'
          ? 'Originals stay in this folder. Scanning only reads them.'
          : 'Managed permission is saved for future approved operations. This version only reads originals.'}
      </Text>
      {identityUnavailable && (
        <Box
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
          role="alert"
          id={identityAlertId}
        >
          <strong>Scanning is unavailable for this folder</strong>
          <Text>
            Stillharbour cannot verify folder identity on this platform or volume. Save &amp; Start
            scanning is disabled to protect your library. On Windows, select an ordinary local NTFS
            folder; network volumes, other filesystems, links and cloud placeholders are not
            supported.
          </Text>
        </Box>
      )}
      <ScanOptions filter={filter} disabled={busy} onChange={setFilter} onError={onError} />
      <Box
        className="folder-actions"
        css={{
          flexBasis: '100%',
          display: 'flex',
          flexWrap: 'wrap',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px 20px',
          '& p': { fontSize: '13px', color: '{colors.fg.muted}', margin: '0', lineHeight: '1.6' },
          '& .primary': { marginTop: '0', marginLeft: 'auto' },
        }}
      >
        <Button type="button" onClick={onCancel} disabled={busy}>
          {embedded ? 'Discard scan changes' : 'Cancel'}
        </Button>
        <Button
          className="primary"
          type="submit"
          disabled={busy || scanRunning || (!root && !selected?.identityAvailable)}
          aria-describedby={identityUnavailable ? identityAlertId : undefined}
        >
          {scanSubmitLabel(busy, scanRunning, Boolean(root))}
        </Button>
      </Box>
    </chakra.form>
  );
}

function scanSubmitLabel(busy: boolean, running: boolean, existing: boolean) {
  if (busy) return 'Starting scan…';
  if (running) return 'Scan running…';
  return existing ? 'Save & Rescan' : 'Save & Start scanning';
}
