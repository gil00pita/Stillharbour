import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { AssetQuery, AppPreferences, LibraryStatus } from '../../bindings';
import type { Section } from '../../library/model';
import { defaultQuery } from '../../library/model';
import {
  QueryFields,
  LibraryFacets,
  RootAvailability,
  ActiveFilters,
} from '@/components/FilterFields';
import type { LibraryController } from '@/hooks/use-library-controller';
export type GalleryDisplay = {
  size: AppPreferences['thumbnailSize'];
  fit: AppPreferences['gridFit'];
};
export function FilterPanel({
  controller: c,
  status,
  section,
  onApply,
  onClose,
  onSettings,
  onRelink,
}: {
  controller: LibraryController;
  status: LibraryStatus;
  section: Section;
  onApply: (query: AssetQuery) => void;
  onClose: () => void;
  onSettings: (id: string) => void;
  onRelink: (id: string) => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draft, setDraft] = useState(c.query);
  const [version, resetFields] = useState(0);
  const [selecting, setSelecting] = useState(false);
  const [selectionError, setSelectionError] = useState<string | null>(null);
  async function selectAll() {
    setSelecting(true);
    setSelectionError(null);
    const success = await c.freeze();
    setSelecting(false);
    if (success) onClose();
    else setSelectionError('Selection could not be created. Try again.');
  }
  const dirty = JSON.stringify(draft) !== JSON.stringify(c.query);
  useEffect(() => {
    const element = dialog.current;
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, []);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (event.currentTarget.reportValidity()) {
      onApply(draft);
      onClose();
    }
  }
  return (
    <chakra.dialog
      ref={dialog}
      id="library-filter-panel"
      className="filter-panel"
      css={{
        position: 'fixed',
        inset: '0 0 0 auto',
        margin: '0',
        width: 'min(380px, 92vw)',
        height: '100vh',
        maxHeight: 'none',
        maxWidth: 'none',
        padding: '0',
        border: '0',
        borderLeft: '1px solid {colors.border}',
        background: '{colors.bg.panel}',
        color: 'inherit',
        '&::backdrop': { background: '#0005' },
        '& > form': { display: 'flex', flexDirection: 'column', height: '100%', minHeight: '0' },
        '& .panel-query-fields': {
          display: 'grid',
          gridTemplateColumns: 'minmax(0, 1fr)',
          gap: '16px',
        },
        '& .form-grid': { display: 'grid', gridTemplateColumns: 'minmax(0, 1fr)', gap: '16px' },
        '& label': { minWidth: '0', maxWidth: '100%' },
        '& select': { minWidth: '0', maxWidth: '100%' },
        '& input': { minWidth: '0', maxWidth: '100%' },
        '& .root-status': { flexWrap: 'wrap' },
        '& .folder-strip': { flexWrap: 'wrap' },
        '& .availability-note': { marginTop: '10px' },
      }}
      aria-labelledby="filter-panel-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!selecting) onClose();
      }}
    >
      <chakra.form onSubmit={submit} aria-busy={selecting}>
        <chakra.header
          className="filter-panel-header"
          css={{
            background: '{colors.bg.toolbar}',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '16px 20px',
            borderBottom: '1px solid {colors.border}',
            '& h2': { margin: '0', fontSize: '18px', letterSpacing: '0' },
          }}
        >
          <Heading as="h2" unstyled id="filter-panel-title">
            Filters
          </Heading>
          <Button type="button" disabled={selecting} onClick={onClose} aria-label="Close filters">
            Close
          </Button>
        </chakra.header>
        <chakra.fieldset
          className="filter-panel-body"
          css={{
            flex: '1',
            minHeight: '0',
            overflowY: 'auto',
            padding: '16px 20px',
            '& section': { padding: '16px 0', borderBottom: '1px solid {colors.border}' },
            '& h3': { fontSize: '13px', margin: '0 0 12px' },
            minWidth: '0',
            border: '0',
            margin: '0',
          }}
          disabled={selecting}
        >
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            {c.page?.total.toLocaleString() ?? '…'} matching photos &amp; videos
          </Text>
          <ActiveFilters
            query={draft}
            filter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
          />
          <chakra.section>
            <Heading as="h3" unstyled>
              Library &amp; sorting
            </Heading>
            <QueryFields
              key={version}
              query={draft}
              status={status}
              filter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
            />
          </chakra.section>
          <chakra.section>
            <Heading as="h3" unstyled>
              Folders &amp; tags
            </Heading>
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Folder (relative)</span>
                </Field.Label>
                <Input
                  maxLength={2048}
                  value={draft.folder}
                  onChange={(event) =>
                    setDraft((value) => ({ ...value, folder: event.target.value }))
                  }
                />
              </label>
            </Field.Root>
            {draft.rootId === c.query.rootId && (
              <LibraryFacets
                section={section}
                query={draft}
                folders={c.folders}
                tags={c.tags}
                onFilter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
              />
            )}
          </chakra.section>
          <chakra.section>
            <Heading as="h3" unstyled>
              Library availability
            </Heading>
            <RootAvailability
              status={status}
              rootId={draft.rootId}
              onSettings={onSettings}
              onRelink={onRelink}
            />
          </chakra.section>
          <Button
            type="button"
            disabled={selecting || dirty || c.loading || c.selection.busy || !c.page?.total}
            onClick={() => void selectAll()}
          >
            Select all results
          </Button>
          {selectionError && <Text role="alert">{selectionError}</Text>}
          {dirty && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Apply changes before selecting all matching results.
            </Text>
          )}
        </chakra.fieldset>
        <chakra.footer
          className="filter-panel-actions"
          css={{
            background: '{colors.bg.toolbar}',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px',
            padding: '16px 20px',
            borderTop: '1px solid {colors.border}',
            '& .primary': { marginTop: '0' },
          }}
        >
          <Button
            type="button"
            disabled={selecting}
            onClick={() => {
              setDraft({
                ...defaultQuery,
                deferPendingPreviews: c.query.deferPendingPreviews ?? false,
              });
              resetFields((value) => value + 1);
            }}
          >
            Reset filters
          </Button>
          <Button type="submit" className="primary" disabled={c.selection.busy || selecting}>
            Apply filters
          </Button>
        </chakra.footer>
      </chakra.form>
    </chakra.dialog>
  );
}
