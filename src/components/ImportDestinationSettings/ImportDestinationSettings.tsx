import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useId, useState, type FormEvent } from 'react';
import type { ImportDestinationPreference, LibraryRoot } from '../../bindings';
import { errorMessage } from '../../ipc';

export function ImportDestinationSettings({
  catalogId,
  preference,
  roots,
  onSave,
}: {
  catalogId: string | null;
  preference: ImportDestinationPreference | null;
  roots: LibraryRoot[];
  onSave: ((rootId: string | null) => Promise<ImportDestinationPreference>) | null;
}) {
  const current = preference?.catalogId === catalogId ? (preference.rootId ?? '') : '';
  const unavailableId = useId();
  const unavailableReason = !catalogId
    ? 'Open a catalog to choose its preferred import destination'
    : !onSave
      ? 'Saving an import destination is unavailable in this session'
      : null;
  const [choice, setChoice] = useState(current);
  const [dirty, setDirty] = useState(false);
  const [pending, setPending] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [observed, setObserved] = useState(current);
  if (observed !== current) {
    setObserved(current);
    if (!dirty) setChoice(current);
  }
  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!catalogId || !onSave || pending) return;
    setPending(true);
    setError(null);
    setSaved(false);
    try {
      const result = await onSave(choice || null);
      if (result.catalogId !== catalogId || result.rootId !== (choice || null))
        throw new Error(
          'The saved destination belongs to another catalog or choice. Refresh and review again.',
        );
      setDirty(false);
      setSaved(true);
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <chakra.section className="settings-section" aria-labelledby="import-destination-title">
      <Heading as="h2" unstyled id="import-destination-title">
        Import destination
      </Heading>
      <Text className="help">
        Suggest a library for new import plans in this catalog. Your base library and browsing
        defaults stay unchanged. You still choose the destination folder and approve each copy;
        unavailable libraries and source folders cannot be used as destinations.
      </Text>
      <chakra.form
        className="settings-import-form"
        aria-label="Import destination preference"
        onSubmit={(event) => void save(event)}
      >
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Preferred import destination</span>
            </Field.Label>
            <NativeSelect
              aria-label="Preferred import destination"
              aria-describedby={unavailableReason ? unavailableId : undefined}
              value={choice}
              disabled={!catalogId || !onSave || pending}
              onChange={(event) => {
                setChoice(event.target.value);
                setDirty(true);
                setSaved(false);
              }}
            >
              <option value="">Choose for each plan</option>
              {choice && !roots.some((root) => root.id === choice) && (
                <option value={choice} disabled>
                  Unavailable saved destination
                </option>
              )}
              {roots.map((root) => (
                <option key={root.id} value={root.id} disabled={root.mode !== 'managed'}>
                  {root.label}
                  {root.mode !== 'managed'
                    ? ' — read-only'
                    : root.availability !== 'online'
                      ? ' — unavailable'
                      : ''}
                </option>
              ))}
            </NativeSelect>
          </label>
        </Field.Root>
        {unavailableReason && (
          <Text className="help" id={unavailableId}>
            {unavailableReason}
          </Text>
        )}
        <Button type="submit" disabled={!catalogId || !onSave || pending || !dirty}>
          {pending ? 'Saving import destination…' : 'Save import destination'}
        </Button>
        {saved && (
          <Text role="status">
            Import destination saved for this catalog. Folder grants and existing plans are
            unchanged.
          </Text>
        )}
        {error && <Text role="alert">{error}</Text>}
      </chakra.form>
    </chakra.section>
  );
}
