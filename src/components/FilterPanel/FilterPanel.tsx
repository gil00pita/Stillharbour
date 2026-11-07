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
    <dialog
      ref={dialog}
      id="library-filter-panel"
      className="filter-panel"
      aria-labelledby="filter-panel-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!selecting) onClose();
      }}
    >
      <chakra.form onSubmit={submit} aria-busy={selecting}>
        <chakra.header className="filter-panel-header">
          <Heading as="h2" unstyled id="filter-panel-title">
            Filters
          </Heading>
          <Button type="button" disabled={selecting} onClick={onClose} aria-label="Close filters">
            Close
          </Button>
        </chakra.header>
        <chakra.fieldset className="filter-panel-body" disabled={selecting}>
          <Text className="help">
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
            <Text className="help">Apply changes before selecting all matching results.</Text>
          )}
        </chakra.fieldset>
        <chakra.footer className="filter-panel-actions">
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
    </dialog>
  );
}
