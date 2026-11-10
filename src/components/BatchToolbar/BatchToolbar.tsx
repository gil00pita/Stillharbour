import { Box, Text, chakra } from '@chakra-ui/react';
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
    <Box
      className={`selection-area ${count ? 'has-selection' : ''}`}
      css={{
        '&.has-selection': {
          position: 'absolute',
          insetInline: '6',
          bottom: '6',
          zIndex: 10,
          maxHeight: '50%',
          overflowY: 'auto',
          padding: '4',
          background: 'bg.elevated',
          backdropFilter: 'blur(12px)',
          border: '1px solid {colors.border}',
          borderRadius: 'panel',
          boxShadow: '0 8px 24px rgb(0 0 0 / 16%)',
          '& .results-heading': { margin: 0 },
        },
      }}
    >
      <Box
        className="results-heading"
        css={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '12px',
          fontSize: '12px',
          margin: '14px 0 10px',
          minHeight: '34px',
          flexWrap: 'wrap',
          '& p': { margin: '0', color: '{colors.fg.muted}' },
          '& > div': { display: 'flex', gap: '6px', flexWrap: 'wrap' },
        }}
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
        <Text>
          {loading ? 'Updating library…' : `${total.toLocaleString()} photos & videos`}
          {count > 0 && ` · ${count.toLocaleString()} selected`}
        </Text>
        <Box>
          {count === 0 ? (
            <Button disabled={!total || busy || loading} onClick={onFreeze}>
              Select all results
            </Button>
          ) : (
            <>
              <Text
                as="span"
                className="selection-scope"
                css={{ fontSize: '11px', color: '{colors.fg.muted}', alignSelf: 'center' }}
              >
                {frozen ? 'All matching results' : 'Explicitly selected photos'}
              </Text>
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
        </Box>
      </Box>
      {editingTags && count > 0 && (
        <chakra.form
          className="batch-tag-form"
          css={{
            padding: '12px 0',
            display: 'flex',
            flexWrap: 'wrap',
            alignItems: 'end',
            gap: '12px',
            '& .tag-field': { flex: '1 1 280px' },
            '& .help': { flexBasis: '100%', margin: '0' },
          }}
          onSubmit={(event) => void saveTags(event)}
        >
          <TagInput
            name="batchTags"
            label="Tags for selected photos"
            placeholder="Add a tag"
            values={tags}
            disabled={busy}
            onChange={setTags}
          />
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Adds to existing catalog tags. Originals and sidecars stay unchanged.
          </Text>
          <Button disabled={busy} type="submit">
            Save tags to selection
          </Button>
        </chakra.form>
      )}
    </Box>
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
