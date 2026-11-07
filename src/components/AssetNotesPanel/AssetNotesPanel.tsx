import { Field, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Textarea } from '@/components/Textarea';
import { useContext, useEffect, useRef, useState } from 'react';
import { MAX_NOTES_DRAFTS, NotesDraftContext, type NotesDraft } from '../../metadata/notesDrafts';
import type { AssetNotes } from '../../bindings';
import { errorMessage } from '../../ipc';
import { metadata } from '../../metadata/ipc';

export function AssetNotesPanel({ assetId }: { assetId: string }) {
  return <Notes key={assetId} assetId={assetId} />;
}
function Notes({ assetId }: { assetId: string }) {
  const sharedDrafts = useContext(NotesDraftContext);
  const [localDrafts] = useState(() => new Map<string, NotesDraft>());
  const drafts = sharedDrafts ?? localDrafts;
  const reloadRequested = useRef(false);
  const [notes, setNotes] = useState<AssetNotes | null>(null);
  const [caption, setCaption] = useState('');
  const [rejected, setRejected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [busy, setBusy] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [attempt, retry] = useState(0);
  useEffect(() => {
    let active = true;
    void metadata({ kind: 'notes', asset_id: assetId }, 'notes')
      .then((value) => {
        if (active) {
          if (reloadRequested.current) drafts.delete(assetId);
          reloadRequested.current = false;
          const draft = drafts.get(assetId);
          setNotes(draft?.baseline ?? value);
          setCaption(draft?.caption ?? value.caption);
          setRejected(draft?.rejected ?? value.rejected);
          setError(null);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      })
      .finally(() => {
        if (active) setReloading(false);
      });
    return () => {
      active = false;
    };
  }, [assetId, attempt, drafts]);
  const dirty = notes && (caption !== notes.caption || rejected !== notes.rejected);
  function edit(nextCaption: string, nextRejected: boolean) {
    if (!notes) return;
    if (nextCaption === notes.caption && nextRejected === notes.rejected) {
      drafts.delete(assetId);
    } else {
      if (!drafts.has(assetId) && drafts.size >= MAX_NOTES_DRAFTS) {
        setError('Save or discard an existing photo notes draft before starting another.');
        return;
      }
      drafts.set(assetId, { baseline: notes, caption: nextCaption, rejected: nextRejected });
    }
    setCaption(nextCaption);
    setRejected(nextRejected);
    setNotice('');
  }
  async function save() {
    if (!notes) return;
    const submittedDraft = drafts.get(assetId);
    setBusy(true);
    setError(null);
    setNotice('');
    try {
      const value = await metadata(
        {
          kind: 'save_notes',
          asset_id: assetId,
          expected_version: notes.version,
          caption,
          rejected,
        },
        'notes',
      );
      if (drafts.get(assetId) === submittedDraft) drafts.delete(assetId);
      setNotes(value);
      setCaption(value.caption);
      setRejected(value.rejected);
      setNotice('Caption and review flag saved in the catalog.');
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }
  return (
    <chakra.section aria-label="Caption and review" className="catalog-notes">
      <Heading as="h3" unstyled>
        Caption &amp; review
      </Heading>
      {error && <Text role="alert">{error}</Text>}
      {!notes && !error ? (
        <Text role="status">Loading catalog notes…</Text>
      ) : (
        notes && (
          <chakra.form
            onSubmit={(event) => {
              event.preventDefault();
              void save();
            }}
          >
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Caption</span>
                </Field.Label>
                <Textarea
                  value={caption}
                  maxLength={4096}
                  rows={4}
                  disabled={busy || reloading}
                  onChange={(event) => edit(event.target.value, rejected)}
                />
              </label>
            </Field.Root>
            <label className="check-row">
              <input
                type="checkbox"
                checked={rejected}
                disabled={busy || reloading}
                onChange={(event) => edit(caption, event.target.checked)}
              />
              Mark as rejected
            </label>
            <Text className="help">
              This review flag stays in the catalog. Original files and sidecars stay unchanged.
            </Text>
            <Button type="submit" disabled={busy || reloading || !dirty}>
              {reloading ? 'Reloading notes…' : busy ? 'Saving notes…' : 'Save caption & review'}
            </Button>
            <details>
              <summary>Embedded keywords · last observed</summary>
              {notes.keywordsAvailable ? (
                <>
                  <List.Root unstyled>
                    {notes.observedKeywords.map((value) => (
                      <List.Item unstyled key={value}>
                        <bdi>{value}</bdi>
                      </List.Item>
                    ))}
                  </List.Root>
                  {!notes.observedKeywords.length && <Text>No keywords were recorded.</Text>}
                  {notes.keywordsTruncated && (
                    <Text>Some metadata values exceed the display limits.</Text>
                  )}
                </>
              ) : (
                <Text>No keyword observation is retained for this item.</Text>
              )}
              <Text className="help">
                These source observations are separate from your catalog tags.
              </Text>
            </details>
          </chakra.form>
        )
      )}
      {(error || dirty) && (
        <>
          <Button
            disabled={busy || reloading}
            onClick={() => {
              setNotice('');
              reloadRequested.current = true;
              setReloading(true);
              retry((value) => value + 1);
            }}
          >
            Reload saved notes
          </Button>
          {dirty && <Text className="help">Reload discards this notes draft.</Text>}
        </>
      )}
      <Text role="status">
        {dirty
          ? 'Unsaved notes. Your draft is kept while browsing this catalog. Save before closing the app or switching catalogs.'
          : notice}
      </Text>
    </chakra.section>
  );
}
