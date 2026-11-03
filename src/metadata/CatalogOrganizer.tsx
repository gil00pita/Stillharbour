import { Button, Input, Select } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import type {
  AssetQuery,
  AssetSelection,
  CatalogCollection,
  CollectionKind,
  TagCategory,
  LibraryRoot,
} from '../bindings';
import { errorMessage } from '../ipc';
import { metadata } from './ipc';
import { defaultQuery } from '../library/model';
import './metadata.css';
import { SavedSearchReview } from './SavedSearchReview';
import { MetadataExportPanel } from './MetadataExportPanel';
import { TagTools } from './TagTools';

export function CatalogOrganizer({
  query,
  roots = [],
  selection,
  selectionCount,
  onBrowse,
  onClose,
  onChanged,
}: {
  query: AssetQuery;
  roots?: LibraryRoot[];
  selection: AssetSelection | null;
  selectionCount: number;
  onBrowse: (query: AssetQuery) => void;
  onClose: () => void;
  onChanged: () => Promise<void>;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const generation = useRef(0);
  const [collections, setCollections] = useState<CatalogCollection[]>([]);
  const [categories, setCategories] = useState<TagCategory[]>([]);
  const [busy, setBusy] = useState(true);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const [name, setName] = useState('');
  const [kind, setKind] = useState<CollectionKind>('album');
  const [selected, setSelected] = useState<string>('');
  const [editingName, setEditingName] = useState('');
  const [categoryName, setCategoryName] = useState('');
  const [parent, setParent] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const current = collections.find((item) => item.id === selected);
  const editingCategory = categories.find((item) => item.id === categoryId);
  useEffect(() => {
    const node = dialog.current;
    const opener = document.activeElement;
    node?.showModal();
    const epoch = ++generation.current;
    void Promise.all([
      metadata({ kind: 'collections' }, 'collections'),
      metadata({ kind: 'tag_categories' }, 'tag_categories'),
    ])
      .then(([c, t]) => {
        if (epoch === generation.current) {
          setCollections(c);
          setCategories(t);
          setLoaded(true);
        }
      })
      .catch((error) => {
        if (epoch === generation.current) setError(errorMessage(error));
      })
      .finally(() => {
        if (epoch === generation.current) setBusy(false);
      });
    return () => {
      generation.current = epoch + 1;
      node?.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  function focusSection(id: string) {
    // Complete the native button activation before moving focus into the scroll region.
    requestAnimationFrame(() => {
      const heading = dialog.current?.querySelector<HTMLElement>(`#${id}`);
      heading?.focus({ preventScroll: true });
      heading?.scrollIntoView({ block: 'start' });
    });
  }
  async function change(operation: () => Promise<void>, message: string) {
    const epoch = generation.current;
    setBusy(true);
    setError(null);
    setNotice('');
    try {
      await operation();
      const [c, t] = await Promise.all([
        metadata({ kind: 'collections' }, 'collections'),
        metadata({ kind: 'tag_categories' }, 'tag_categories'),
      ]);
      if (epoch === generation.current) {
        setCollections(c);
        setCategories(t);
        setLoaded(true);
        setNotice(message);
        await onChanged();
      }
    } catch (error) {
      if (epoch === generation.current) setError(errorMessage(error));
    } finally {
      if (epoch === generation.current) setBusy(false);
    }
  }
  function openCollection(value: CatalogCollection) {
    if (value.archived) return;
    onBrowse(value.kind === 'album' ? { ...defaultQuery, albumId: value.id } : value.query!);
    onClose();
  }
  async function membership(remove: boolean) {
    if (!current || !selection) return;
    const result = await metadata(
      {
        kind: 'collection_members',
        id: current.id,
        expected_version: current.version,
        expected_count: selectionCount,
        selection,
        remove,
      },
      'membership',
    );
    setNotice(
      `${result.changed.toLocaleString()} album memberships ${remove ? 'removed' : 'added'}.`,
    );
  }
  return (
    <dialog
      ref={dialog}
      className="catalog-organizer"
      aria-labelledby="catalog-organizer-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <header>
        <div>
          <h2 id="catalog-organizer-title">Albums, searches &amp; tags</h2>
          <p>Organize your catalog. Originals stay in their folders.</p>
          <nav className="catalog-section-links" aria-label="Organization sections">
            <Button onClick={() => focusSection('collections-title')}>Albums &amp; searches</Button>
            <Button onClick={() => focusSection('categories-title')}>Tag categories</Button>
          </nav>
        </div>
        <Button disabled={busy} onClick={onClose}>
          Close
        </Button>
      </header>
      <div className="catalog-organizer-scroll">
        {error && (
          <div role="alert">
            <p>{error}</p>
            <Button
              disabled={busy}
              onClick={() =>
                void change(async () => {}, 'Catalog refreshed. Your drafts are retained.')
              }
            >
              Reload catalog organization
            </Button>
          </div>
        )}
        <p role="status">{busy ? 'Updating catalog…' : notice}</p>
        <fieldset disabled={busy} className="catalog-organizer-body">
          <section aria-labelledby="collections-title">
            <h3 id="collections-title" tabIndex={-1}>
              Virtual albums &amp; saved searches
            </h3>
            <form
              className="catalog-form"
              onSubmit={(event) => {
                event.preventDefault();
                void change(async () => {
                  const value = await metadata(
                    {
                      kind: 'create_collection',
                      collection_kind: kind,
                      name,
                      query: kind === 'search' ? query : null,
                    },
                    'collection',
                  );
                  setSelected(value.id);
                  setEditingName(value.name);
                  setName('');
                }, 'Catalog collection created.');
              }}
            >
              <label>
                New collection name
                <Input
                  required
                  maxLength={128}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </label>
              <label>
                Collection type
                <Select value={kind} onChange={(e) => setKind(e.target.value as CollectionKind)}>
                  <option value="album">Virtual album</option>
                  <option value="search">Saved search · current filters</option>
                </Select>
              </label>
              <Button type="submit">Create collection</Button>
            </form>
            <label>
              Catalog collection
              <Select
                value={selected}
                onChange={(e) => {
                  setSelected(e.target.value);
                  setEditingName(collections.find((v) => v.id === e.target.value)?.name ?? '');
                }}
              >
                <option value="">Choose a collection</option>
                {collections.map((value) => (
                  <option key={value.id} value={value.id}>
                    {value.name} ·{' '}
                    {value.kind === 'album' ? `${value.memberCount} members` : 'saved search'}
                    {value.archived ? ' · Archived' : ''}
                  </option>
                ))}
              </Select>
            </label>
            {current && (
              <div className="catalog-collection-detail">
                <div className="catalog-actions">
                  <Button disabled={current.archived} onClick={() => openCollection(current)}>
                    View {current.kind === 'album' ? 'album' : 'saved search'}
                  </Button>
                  <Button
                    onClick={() =>
                      void change(
                        async () => {
                          await metadata(
                            {
                              kind: 'archive_collection',
                              id: current.id,
                              expected_version: current.version,
                              archived: !current.archived,
                            },
                            'collection',
                          );
                        },
                        current.archived
                          ? 'Collection restored.'
                          : 'Collection archived; memberships retained.',
                      )
                    }
                  >
                    {current.archived ? 'Restore collection' : 'Archive collection'}
                  </Button>
                </div>
                <form
                  className="catalog-form"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void change(async () => {
                      await metadata(
                        {
                          kind: 'update_collection',
                          id: current.id,
                          expected_version: current.version,
                          name: editingName,
                          query: current.query,
                        },
                        'collection',
                      );
                    }, 'Collection renamed.');
                  }}
                >
                  <label>
                    Collection name
                    <Input
                      required
                      maxLength={128}
                      value={editingName}
                      onChange={(e) => setEditingName(e.target.value)}
                    />
                  </label>
                  <Button type="submit">Save collection name</Button>
                </form>
                {current.kind === 'search' && (
                  <>
                    <p className="help">
                      Saved searches contain filter data. They grant no file permissions or
                      scheduled actions.
                    </p>
                    <SavedSearchReview
                      key={`${current.id}:${current.version}`}
                      current={current}
                      query={query}
                      roots={roots}
                      collections={collections}
                      onApply={(replacement) =>
                        change(async () => {
                          await metadata(
                            {
                              kind: 'update_collection',
                              id: current.id,
                              expected_version: current.version,
                              name: current.name,
                              query: replacement,
                            },
                            'collection',
                          );
                        }, 'Saved search updated from the reviewed filters.')
                      }
                    />
                  </>
                )}
                {current.kind === 'album' && (
                  <>
                    <p>{selectionCount.toLocaleString()} photos selected in the library.</p>
                    <div className="catalog-actions">
                      <Button
                        disabled={!selection || current.archived}
                        onClick={() =>
                          void change(
                            () => membership(false),
                            'Selected photos added to the virtual album.',
                          )
                        }
                      >
                        Add selection to album
                      </Button>
                      <Button
                        disabled={!selection || current.archived}
                        onClick={() =>
                          void change(
                            () => membership(true),
                            'Selected photos removed from this album only.',
                          )
                        }
                      >
                        Remove selection from album
                      </Button>
                    </div>
                  </>
                )}
              </div>
            )}
            {loaded && !collections.length && !busy && <p>No albums or saved searches yet.</p>}
            <details>
              <summary>About albums &amp; saved searches</summary>
              <p className="help">
                An album groups photos without moving them. A saved search finds photos using saved
                filters.
              </p>
              <p className="help">
                Up to 200 collections, including archived entries. An archived album keeps its
                photos for restoration.
              </p>
            </details>
          </section>
          <section aria-labelledby="categories-title">
            <h3 id="categories-title" tabIndex={-1}>
              Tag categories
            </h3>
            <p className="help">
              Group related tags under a parent. Choosing a parent also includes its child tags.
            </p>
            <label>
              Category to arrange
              <Select
                value={categoryId}
                onChange={(e) => {
                  setCategoryId(e.target.value);
                  const value = categories.find((n) => n.id === e.target.value);
                  setCategoryName(value?.label ?? '');
                  setParent(value?.parentId ?? '');
                }}
              >
                <option value="">New category</option>
                {categories.map((node) => (
                  <option key={node.id} value={node.id}>
                    {node.label}
                  </option>
                ))}
              </Select>
            </label>
            <form
              className="catalog-form"
              onSubmit={(event) => {
                event.preventDefault();
                void change(async () => {
                  await metadata(
                    {
                      kind: 'save_tag_category',
                      id: editingCategory?.id ?? null,
                      expected_version: editingCategory?.version ?? null,
                      label: categoryName,
                      parent_id: parent || null,
                    },
                    'tag_category',
                  );
                }, 'Tag category arrangement saved.');
              }}
            >
              <label>
                Tag label
                <Input
                  required
                  maxLength={128}
                  readOnly={Boolean(editingCategory)}
                  value={categoryName}
                  onChange={(e) => setCategoryName(e.target.value)}
                />
              </label>
              <label>
                Parent category
                <Select value={parent} onChange={(e) => setParent(e.target.value)}>
                  <option value="">No parent</option>
                  {categories
                    .filter((node) => node.id !== categoryId)
                    .map((node) => (
                      <option key={node.id} value={node.id}>
                        {node.label}
                      </option>
                    ))}
                </Select>
              </label>
              <Button type="submit">
                {editingCategory ? 'Save category parent' : 'Create tag category'}
              </Button>
            </form>
            <ul className="catalog-categories">
              {categories.map((node) => (
                <li key={node.id}>
                  <span>
                    {node.parentId
                      ? `${categories.find((parent) => parent.id === node.parentId)?.label ?? 'Unavailable parent'} → `
                      : ''}
                    <bdi>{node.label}</bdi>
                  </span>
                  <Button
                    aria-label={`View category ${node.label}`}
                    onClick={() => {
                      onBrowse({ ...query, tag: node.label, tagDescendants: true });
                      onClose();
                    }}
                  >
                    View category
                  </Button>
                </li>
              ))}
            </ul>
            <details>
              <summary>Tag matching &amp; limits</summary>
              <p className="help">
                Tags match exactly: case, Unicode and labels containing slashes remain distinct.
              </p>
              <p className="help">
                Up to 512 categories and sixteen levels. Renaming/merging tag labels and sidecar
                export require their separate reviewed workflows.
              </p>
            </details>
          </section>
          <TagTools
            onChanged={() => change(async () => {}, 'Tag metadata refreshed.')}
            onBusy={setBusy}
          />
        </fieldset>
        <MetadataExportPanel
          selection={selection}
          selectionCount={selectionCount}
          onBusy={setBusy}
        />
      </div>
    </dialog>
  );
}
