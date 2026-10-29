import { useEffect, useRef, useState, type FormEvent } from 'react';
import type { AssetQuery, AppPreferences, LibraryStatus } from '../bindings';
import type { Section, ViewMode } from './model';
import { defaultQuery } from './model';
import { QueryFields, LibraryFacets, RootAvailability, ActiveFilters } from './FilterFields';
import { BrowserControls } from './BrowserControls';
import type { LibraryController } from './useLibraryController';
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
      <form onSubmit={submit} aria-busy={selecting}>
        <header className="filter-panel-header">
          <h2 id="filter-panel-title">Filters</h2>
          <button type="button" disabled={selecting} onClick={onClose} aria-label="Close filters">
            Close
          </button>
        </header>
        <fieldset className="filter-panel-body" disabled={selecting}>
          <p className="help">
            {c.page?.total.toLocaleString() ?? '…'} matching photos &amp; videos
          </p>
          <ActiveFilters
            query={draft}
            filter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
          />
          <section>
            <h3>Library &amp; sorting</h3>
            <QueryFields
              key={version}
              query={draft}
              status={status}
              filter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
            />
          </section>
          <section>
            <h3>Display</h3>
            <BrowserControls
              mode={draftMode}
              size={draftDisplay.size}
              fit={draftDisplay.fit}
              onMode={setMode}
              onSize={(size) => setDisplay((value) => ({ ...value, size }))}
              onFit={(fit) => setDisplay((value) => ({ ...value, fit }))}
            />
          </section>
          <section>
            <h3>Folders &amp; tags</h3>
            <label>
              Folder (relative)
              <input
                maxLength={2048}
                value={draft.folder}
                onChange={(event) =>
                  setDraft((value) => ({ ...value, folder: event.target.value }))
                }
              />
            </label>
            {draft.rootId === c.query.rootId && (
              <LibraryFacets
                section={section}
                query={draft}
                folders={c.folders}
                tags={c.tags}
                onFilter={(patch) => setDraft((value) => ({ ...value, ...patch }))}
              />
            )}
          </section>
          <section>
            <h3>Library availability</h3>
            <RootAvailability
              status={status}
              rootId={draft.rootId}
              onSettings={onSettings}
              onRelink={onRelink}
            />
          </section>
          <button
            type="button"
            disabled={selecting || dirty || c.loading || c.selection.busy || !c.page?.total}
            onClick={() => void selectAll()}
          >
            Select all results
          </button>
          {selectionError && <p role="alert">{selectionError}</p>}
          {dirty && <p className="help">Apply changes before selecting all matching results.</p>}
        </fieldset>
        <footer className="filter-panel-actions">
          <button
            type="button"
            disabled={selecting}
            onClick={() => {
              setDraft({ ...defaultQuery });
              resetFields((value) => value + 1);
            }}
          >
            Reset filters
          </button>
          <button type="submit" className="primary" disabled={c.selection.busy || selecting}>
            Apply filters
          </button>
        </footer>
      </form>
    </dialog>
  );
}
