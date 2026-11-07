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
      aria-labelledby="scan-settings-title"
    >
      <Heading as="h2" unstyled id="scan-settings-title" ref={heading} tabIndex={-1}>
        Scan settings
      </Heading>
      <Text className="help">
        Saved for the selected library only. Apply changes with Save &amp; Rescan; originals stay
        unchanged.
      </Text>
      {!root ? (
        <Text className="help">Add a photo folder to configure its scan settings.</Text>
      ) : (
        <>
          <Field.Root asChild unstyled>
            <label className="scan-library-selector">
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
            </label>
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
            <Text className="error" role="alert">
              {error}
            </Text>
          )}
          {started && (
            <Text className="settings-saved" role="status">
              Scan started. Originals stay unchanged.
            </Text>
          )}
        </>
      )}
    </chakra.section>
  );
}
