import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { AssetQuery, AppPreferences, LibraryStatus } from '../../bindings';
import type { Section, ViewMode } from '../../library/model';
import { defaultQuery } from '../../library/model';
import {
  QueryFields,
  LibraryFacets,
  RootAvailability,
  ActiveFilters,
} from '@/components/FilterFields';
import { BrowserControls } from '@/components/BrowserControls';
import type { LibraryController } from '@/hooks/use-library-controller';
export type GalleryDisplay = {
  size: AppPreferences['thumbnailSize'];
  fit: AppPreferences['gridFit'];
};
export function FilterPanel({
  controller: c,
  status,
  section,
  mode,
  display,
  onApply,
  onClose,
  onSettings,
  onRelink,
}: {
  controller: LibraryController;
  status: LibraryStatus;
  section: Section;
  mode: ViewMode;
  display: GalleryDisplay;
  onApply: (query: AssetQuery, mode: ViewMode, display: GalleryDisplay) => void;
  onClose: () => void;
  onSettings: (id: string) => void;
  onRelink: (id: string) => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draft, setDraft] = useState(c.query);
  const [draftMode, setMode] = useState(mode);
  const [draftDisplay, setDisplay] = useState(display);
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
      onApply(draft, draftMode, draftDisplay);
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
        '& .view-switch': { display: 'flex', flexWrap: 'wrap', gap: '6px' },
        '& .thumbnail-control': { width: '100%', margin: '8px 0 0' },
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
              Display
            </Heading>
            <BrowserControls
              mode={draftMode}
              size={draftDisplay.size}
              fit={draftDisplay.fit}
              onMode={setMode}
              onSize={(size) => setDisplay((value) => ({ ...value, size }))}
              onFit={(fit) => setDisplay((value) => ({ ...value, fit }))}
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
          <Button
            type="submit"
            className="primary"
            css={{
              marginTop: '28px',
              border: '0',
              background: '{colors.brand.solid}',
              color: '{colors.brand.contrast}',
              fontWeight: '600',
              fontSize: '13px',
              lineHeight: '20px',
              borderRadius: 'var(--control-radius)',
              padding: '12px 24px',
              minHeight: '44px',
              transition: 'background-color 140ms var(--ui-ease)',
              '&:hover:enabled': { background: '{colors.brand.hover}' },
            }}
            disabled={c.selection.busy || selecting}
          >
            Apply filters
          </Button>
        </chakra.footer>
      </chakra.form>
    </chakra.dialog>
  );
}
