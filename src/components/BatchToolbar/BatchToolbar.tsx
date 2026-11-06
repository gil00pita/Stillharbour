import { Button } from '@/components/Button';
import { useState, type FormEvent } from 'react';
import { TagInput } from '@/components/TagInput';

export function BatchToolbar({
  loading,
  total,
  count,
  busy,
  frozen,
  canCompare,
  onFreeze,
  onFavorite,
  onClear,
  onTags,
  onCompare,
  onOrganization,
  onAlbum,
}: {
  loading: boolean;
  total: number;
  count: number;
  busy: boolean;
  frozen: boolean;
  canCompare: boolean;
  onFreeze: () => void;
  onFavorite: () => void;
  onClear: () => void;
  onTags: (tags: string[]) => Promise<boolean>;
  onCompare: () => void;
  onOrganization?: () => void;
  onAlbum?: () => void;
}) {
  const [editingTags, setEditingTags] = useState(false);
  const [tags, setTags] = useState<string[]>([]);
  async function saveTags(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const pending = new FormData(event.currentTarget).get('batchTags');
    const values = [
      ...new Set([
        ...tags,
        ...(typeof pending === 'string' && pending.trim() ? [pending.trim()] : []),
      ]),
    ];
    if (!values.length) return;
    if (await onTags(values)) {
      setTags([]);
      setEditingTags(false);
    }
  }
  return (
    <div className={`selection-area ${count ? 'has-selection' : ''}`}>
      <div
        className="results-heading"
        role={count ? 'toolbar' : undefined}
        aria-label={count ? 'Selected photo actions' : undefined}
        onKeyDown={(event) => {
          if (!['ArrowLeft', 'ArrowRight'].includes(event.key)) return;
          const buttons = Array.from(
            event.currentTarget.querySelectorAll<HTMLButtonElement>('button:enabled'),
          );
          const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
          if (index < 0) return;
          event.preventDefault();
          buttons[
            (index + (event.key === 'ArrowRight' ? 1 : -1) + buttons.length) % buttons.length
          ]?.focus();
        }}
      >
        <p>
          {loading ? 'Updating library…' : `${total.toLocaleString()} photos & videos`}
          {count > 0 && ` · ${count.toLocaleString()} selected`}
        </p>
        <div>
          {count === 0 ? (
            <Button disabled={!total || busy || loading} onClick={onFreeze}>
              Select all results
            </Button>
          ) : (
            <>
              <span className="selection-scope">
                {frozen ? 'All matching results' : 'Explicitly selected photos'}
              </span>
              <Button disabled={busy} onClick={() => setEditingTags((value) => !value)}>
                Add tags
              </Button>
              <Button disabled={busy} onClick={onFavorite}>
                Favorite selected
              </Button>
              <Button
                disabled={busy || !canCompare}
                title={
                  canCompare
                    ? 'Open a read-only visual comparison'
                    : 'Choose 2–4 individual photos to compare'
                }
                onClick={onCompare}
              >
                Compare
              </Button>
              <Button
                disabled={busy || !onAlbum}
                title="Choose a virtual album for this selection"
                onClick={onAlbum}
              >
                Add to album
              </Button>
              <OrganizationAction
                busy={busy}
                frozen={frozen}
                count={count}
                onPreview={onOrganization}
              />
              <Button disabled title="Export requires a separate reviewed preservation workflow.">
                Export
              </Button>
              <Button
                disabled={busy}
                onClick={() => {
                  setEditingTags(false);
                  onClear();
                }}
              >
                Clear selection
              </Button>
            </>
          )}
        </div>
      </div>
      {editingTags && count > 0 && (
        <form className="batch-tag-form" onSubmit={(event) => void saveTags(event)}>
          <TagInput
            name="batchTags"
            label="Tags for selected photos"
            placeholder="Add a tag"
            values={tags}
            disabled={busy}
            onChange={setTags}
          />
          <p className="help">
            Adds to existing catalog tags. Originals and sidecars stay unchanged.
          </p>
          <Button disabled={busy} type="submit">
            Save tags to selection
          </Button>
        </form>
      )}
    </div>
  );
}

function OrganizationAction({
  busy,
  frozen,
  count,
  onPreview,
}: {
  busy: boolean;
  frozen: boolean;
  count: number;
  onPreview: (() => void) | undefined;
}) {
  const unsupportedScope = frozen || count > 240;
  return (
    <Button
      disabled={busy || unsupportedScope || !onPreview}
      title={
        unsupportedScope
          ? 'Choose up to 240 individual photos for a template preview'
          : 'Preview proposed names without changing originals'
      }
      onClick={onPreview}
    >
      Preview organization
    </Button>
  );
}
