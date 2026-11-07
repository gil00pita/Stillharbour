import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type { LibraryRoot, RootMode, ScanFilter } from '../../bindings';
import { errorMessage } from '../../ipc';
import { RootSetup } from '@/components/RootSetup';

export function ScanSettingsGroup({
  roots,
  initialRootId,
  busy,
  onRescan,
  onError,
}: {
  roots: LibraryRoot[];
  initialRootId: string | null;
  busy: boolean;
  onRescan: (id: string, mode: RootMode, filter: ScanFilter) => Promise<void>;
  onError: (error: unknown) => void;
}) {
  const [rootId, setRootId] = useState(initialRootId);
  const [version, reset] = useState(0);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [started, setStarted] = useState(false);
  const heading = useRef<HTMLHeadingElement>(null);
  const root = roots.find((value) => value.id === rootId) ?? roots[0];
  useEffect(() => {
    if (initialRootId) heading.current?.focus();
  }, [initialRootId]);
  return (
    <chakra.section
      className="settings-section settings-scan-group"
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
      aria-labelledby="scan-settings-title"
    >
      <Heading as="h2" unstyled id="scan-settings-title" ref={heading} tabIndex={-1}>
        Scan settings
      </Heading>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Saved for the selected library only. Apply changes with Save &amp; Rescan; originals stay
        unchanged.
      </Text>
      {!root ? (
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Add a photo folder to configure its scan settings.
        </Text>
      ) : (
        <>
          <Field.Root asChild unstyled>
            <chakra.label
              className="scan-library-selector"
              css={{ display: 'block', maxWidth: '420px', margin: '20px 0 12px' }}
            >
              <Field.Label asChild unstyled>
                <span>Library to configure</span>
              </Field.Label>
              <NativeSelect
                aria-label="Library to configure"
                value={root.id}
                disabled={busy || saving}
                onChange={(event) => {
                  setRootId(event.target.value);
                  setError(null);
                  setStarted(false);
                }}
              >
                {roots.map((value) => (
                  <option key={value.id} value={value.id}>
                    {value.label}
                  </option>
                ))}
              </NativeSelect>
            </chakra.label>
          </Field.Root>
          <RootSetup
            key={`${root.id}:${version}`}
            embedded
            root={root}
            busy={busy || saving}
            scanRunning={root.availability === 'scanning'}
            onError={onError}
            onCancel={() => {
              reset((value) => value + 1);
              setError(null);
              setStarted(false);
            }}
            onSave={async (_label, mode, filter) => {
              setSaving(true);
              setError(null);
              setStarted(false);
              try {
                await onRescan(root.id, mode, filter);
                setStarted(true);
              } catch (error) {
                setError(errorMessage(error));
              } finally {
                setSaving(false);
              }
            }}
          />
          {error && (
            <Text
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
            >
              {error}
            </Text>
          )}
          {started && (
            <Text
              className="settings-saved"
              css={{ color: '{colors.brand.solid}', marginTop: '16px' }}
              role="status"
            >
              Scan started. Originals stay unchanged.
            </Text>
          )}
        </>
      )}
    </chakra.section>
  );
}
