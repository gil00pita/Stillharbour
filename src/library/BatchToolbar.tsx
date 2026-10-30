import { useState, type FormEvent } from 'react';
import { TagInput } from '../components/TagInput';

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
            <button disabled={!total || busy || loading} onClick={onFreeze}>
              Select all results
            </button>
          ) : (
            <>
              <span className="selection-scope">
                {frozen ? 'All matching results' : 'Explicitly selected photos'}
              </span>
              <button disabled={busy} onClick={() => setEditingTags((value) => !value)}>
                Add tags
              </button>
              <button disabled={busy} onClick={onFavorite}>
                Favorite selected
              </button>
              <button
                disabled={busy || !canCompare}
                title={
                  canCompare
                    ? 'Open a read-only visual comparison'
                    : 'Choose 2–4 individual photos to compare'
                }
                onClick={onCompare}
              >
                Compare
              </button>
              <button disabled title="Albums are not available yet; tags are saved in the catalog.">
                Add to album
              </button>
              <OrganizationAction
                busy={busy}
                frozen={frozen}
                count={count}
                onPreview={onOrganization}
              />
              <button disabled title="Export requires a separate reviewed preservation workflow.">
                Export
              </button>
              <button
                disabled={busy}
                onClick={() => {
                  setEditingTags(false);
                  onClear();
                }}
              >
                Clear selection
              </button>
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
          <button disabled={busy} type="submit">
            Save tags to selection
          </button>
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
    <button
      disabled={busy || unsupportedScope || !onPreview}
      title={
        unsupportedScope
          ? 'Choose up to 240 individual photos for a template preview'
          : 'Preview proposed names without changing originals'
      }
      onClick={onPreview}
    >
      Preview organization
    </button>
  );
}
