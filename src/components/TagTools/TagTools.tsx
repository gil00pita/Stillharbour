import { Field, Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { useEffect, useState } from 'react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import type { TagEditReview, TagHistoryEntry } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { metadata } from '../../metadata/ipc';

export function TagTools({
  onChanged,
  onBusy,
}: {
  onChanged: () => Promise<void>;
  onBusy: (busy: boolean) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <details onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary>Rename, merge, remove & undo tags</summary>
      {open && <TagEditor onChanged={onChanged} onBusy={onBusy} />}
    </details>
  );
}
function TagEditor({
  onChanged,
  onBusy,
}: {
  onChanged: () => Promise<void>;
  onBusy: (busy: boolean) => void;
}) {
  const [from, setFrom] = useState('');
  const [to, setTo] = useState('');
  const [remove, setRemove] = useState(false);
  const [review, setReview] = useState<TagEditReview | null>(null);
  const [history, setHistory] = useState<TagHistoryEntry[]>([]);
  const [revision, setRevision] = useState('');
  const [attempt, retry] = useState(0);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const waiting = busy || loading;
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  useEffect(() => {
    let active = true;
    void Promise.all([
      metadata({ kind: 'tag_history' }, 'tag_history'),
      libraryData({ kind: 'status' }, 'status'),
    ])
      .then(([history, status]) => {
        if (active) {
          setHistory(history);
          setRevision(status.revision);
        }
      })
      .catch((error) => {
        if (active) setError(errorMessage(error));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [attempt]);
  async function run(operation: () => Promise<void>) {
    setBusy(true);
    onBusy(true);
    setError(null);
    setNotice('');
    try {
      await operation();
      await onChanged();
      const [freshHistory, status] = await Promise.all([
        metadata({ kind: 'tag_history' }, 'tag_history'),
        libraryData({ kind: 'status' }, 'status'),
      ]);
      setHistory(freshHistory);
      setRevision(status.revision);
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setBusy(false);
      onBusy(false);
    }
  }
  return (
    <chakra.section aria-label="Tag changes">
      <Text className="help">
        These catalog-only changes affect every exact matching label, including offline media and
        category relationships. Case, Unicode and literal slashes stay distinct. Original files
        remain unchanged.
      </Text>
      <chakra.form
        onSubmit={(event) => {
          event.preventDefault();
          void run(async () => {
            setReview(
              await metadata(
                { kind: 'tag_edit_preview', from, to: remove ? null : to },
                'tag_edit_review',
              ),
            );
          });
        }}
      >
        <Field.Root required>
          <Field.Label>Exact tag to change</Field.Label>
          <Input
            required
            value={from}
            disabled={waiting}
            maxLength={128}
            onChange={(e) => {
              setFrom(e.target.value);
              setReview(null);
            }}
          />
        </Field.Root>
        <label className="check-row">
          <input
            type="checkbox"
            checked={remove}
            disabled={waiting}
            onChange={(e) => {
              setRemove(e.target.checked);
              setReview(null);
            }}
          />
          Remove this label from the catalog
        </label>
        {!remove && (
          <Field.Root required>
            <Field.Label>New or existing exact label</Field.Label>
            <Input
              required
              value={to}
              maxLength={128}
              disabled={waiting}
              onChange={(e) => {
                setTo(e.target.value);
                setReview(null);
              }}
            />
          </Field.Root>
        )}
        <Button type="submit" disabled={waiting}>
          Preview tag change
        </Button>
      </chakra.form>
      {review && (
        <Box>
          <Text>
            {review.affectedCount.toLocaleString()} media ·{' '}
            {review.to === null ? 'remove' : review.merges ? 'merge' : 'rename'}{' '}
            <bdi>{review.from}</bdi>
            {review.to !== null && (
              <>
                {' '}
                → <bdi>{review.to}</bdi>
              </>
            )}
            . Category relationships are included; incompatible cycles are blocked.
          </Text>
          <Button
            disabled={waiting}
            onClick={() =>
              void run(async () => {
                await metadata(
                  { kind: 'tag_edit_apply', id: review.id, digest: review.digest },
                  'tag_history_entry',
                );
                setReview(null);
                setNotice('Reviewed tag change saved. History is retained.');
              })
            }
          >
            Apply reviewed tag change
          </Button>
        </Box>
      )}
      <Heading as="h4" unstyled>
        Recent tag history
      </Heading>
      <List.Root unstyled>
        {history.map((entry) => (
          <List.Item unstyled key={entry.id}>
            {entry.affectedCount.toLocaleString()} media · {entry.createdAt}
            <Button
              disabled={waiting || !revision}
              onClick={() =>
                void run(async () => {
                  await metadata(
                    { kind: 'tag_undo', id: entry.id, expected_revision: revision },
                    'tag_history_entry',
                  );
                  setNotice('Tag state restored as a new history entry.');
                })
              }
            >
              Undo tag change {entry.id.slice(0, 8)}
            </Button>
          </List.Item>
        ))}
      </List.Root>
      {error && (
        <>
          <Text role="alert">{error}</Text>
          <Button
            disabled={waiting}
            onClick={() => {
              setLoading(true);
              retry((x) => x + 1);
            }}
          >
            Reload tag history
          </Button>
        </>
      )}
      <Text role="status">{busy ? 'Updating tag review…' : notice}</Text>
    </chakra.section>
  );
}
