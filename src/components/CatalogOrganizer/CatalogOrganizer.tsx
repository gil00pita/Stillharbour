import { Field, Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type {
  AssetQuery,
  AssetSelection,
  CatalogCollection,
  CollectionKind,
  TagCategory,
  LibraryRoot,
} from '../../bindings';
import { errorMessage } from '../../ipc';
import { metadata } from '../../metadata/ipc';
import { defaultQuery } from '../../library/model';
import { SavedSearchReview } from '@/components/SavedSearchReview';
import { MetadataExportPanel } from '@/components/MetadataExportPanel';
import { TagTools } from '@/components/TagTools';

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
    <chakra.dialog
      ref={dialog}
      className="catalog-organizer"
      css={{
        width: 'min(880px, calc(100vw - 32px))',
        maxHeight: 'calc(100vh - 32px)',
        overflow: 'hidden',
        border: '1px solid {colors.border}',
        borderRadius: '12px',
        padding: '0',
        background: '{colors.bg.panel}',
        color: 'inherit',
        '&[open]': { display: 'flex', flexDirection: 'column' },
        '&::backdrop': { background: '#0009' },
        '& > header': {
          display: 'flex',
          justifyContent: 'space-between',
          gap: '20px',
          alignItems: 'start',
          flexShrink: '0',
          padding: '20px 24px',
          borderBottom: '1px solid {colors.border}',
        },
        '& > header button': { flexShrink: '0' },
        '& h2': { margin: '0 0 8px' },
        '& > header p': { margin: '0', color: '{colors.fg.muted}' },
        '& label': { display: 'grid', gap: '6px', margin: '10px 0' },
        '& label.check-row': { display: 'flex', alignItems: 'center' },
        '@media (max-width: 760px)': { '& > header': { padding: '16px' } },
      }}
      aria-labelledby="catalog-organizer-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <chakra.header>
        <Box>
          <Heading as="h2" unstyled id="catalog-organizer-title">
            Albums, searches &amp; tags
          </Heading>
          <Text>Organize your catalog. Originals stay in their folders.</Text>
          <chakra.nav
            className="catalog-section-links"
            css={{ display: 'flex', flexWrap: 'wrap', gap: '8px', marginTop: '16px' }}
            aria-label="Organization sections"
          >
            <Button onClick={() => focusSection('collections-title')}>Albums &amp; searches</Button>
            <Button onClick={() => focusSection('categories-title')}>Tag categories</Button>
          </chakra.nav>
        </Box>
        <Button disabled={busy} onClick={onClose}>
          Close
        </Button>
      </chakra.header>
      <Box
        className="catalog-organizer-scroll"
        css={{
          minHeight: '0',
          overflowY: 'auto',
          overscrollBehavior: 'contain',
          padding: '0 24px 24px',
          scrollbarGutter: 'stable',
          '@media (max-width: 760px)': { padding: '0 16px 16px' },
        }}
      >
        {error && (
          <Box role="alert">
            <Text>{error}</Text>
            <Button
              disabled={busy}
              onClick={() =>
                void change(async () => {}, 'Catalog refreshed. Your drafts are retained.')
              }
            >
              Reload catalog organization
            </Button>
          </Box>
        )}
        <Text role="status">{busy ? 'Updating catalog…' : notice}</Text>
        <chakra.fieldset
          disabled={busy}
          className="catalog-organizer-body"
          css={{
            border: '0',
            padding: '0',
            margin: '0',
            display: 'grid',
            gap: '24px',
            minWidth: '0',
            '& > section': { borderTop: '1px solid {colors.border}', paddingTop: '12px' },
            '& h3:focus-visible': {
              outline: '2px solid {colors.brand.solid}',
              outlineOffset: '4px',
            },
            '& h3': { scrollMarginBlock: '16px' },
          }}
        >
          <chakra.section aria-labelledby="collections-title">
            <Heading as="h3" unstyled id="collections-title" tabIndex={-1}>
              Virtual albums &amp; saved searches
            </Heading>
            <chakra.form
              className="catalog-form"
              css={{
                display: 'flex',
                flexWrap: 'wrap',
                gap: '12px',
                alignItems: 'end',
                margin: '12px 0',
                '& label': { flex: '1', minWidth: '170px' },
                '& button': { marginBottom: '10px' },
                '@media (max-width: 760px)': { display: 'grid' },
              }}
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
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>New collection name</span>
                  </Field.Label>
                  <Input
                    required
                    maxLength={128}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                  />
                </label>
              </Field.Root>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Collection type</span>
                  </Field.Label>
                  <NativeSelect
                    value={kind}
                    onChange={(e) => setKind(e.target.value as CollectionKind)}
                  >
                    <option value="album">Virtual album</option>
                    <option value="search">Saved search · current filters</option>
                  </NativeSelect>
                </label>
              </Field.Root>
              <Button type="submit">Create collection</Button>
            </chakra.form>
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Catalog collection</span>
                </Field.Label>
                <NativeSelect
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
                </NativeSelect>
              </label>
            </Field.Root>
            {current && (
              <Box className="catalog-collection-detail">
                <Box
                  className="catalog-actions"
                  css={{ display: 'flex', flexWrap: 'wrap', gap: '10px', margin: '12px 0' }}
                >
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
                </Box>
                <chakra.form
                  className="catalog-form"
                  css={{
                    display: 'flex',
                    flexWrap: 'wrap',
                    gap: '12px',
                    alignItems: 'end',
                    margin: '12px 0',
                    '& label': { flex: '1', minWidth: '170px' },
                    '& button': { marginBottom: '10px' },
                    '@media (max-width: 760px)': { display: 'grid' },
                  }}
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
                  <Field.Root asChild unstyled>
                    <label>
                      <Field.Label asChild unstyled>
                        <span>Collection name</span>
                      </Field.Label>
                      <Input
                        required
                        maxLength={128}
                        value={editingName}
                        onChange={(e) => setEditingName(e.target.value)}
                      />
                    </label>
                  </Field.Root>
                  <Button type="submit">Save collection name</Button>
                </chakra.form>
                {current.kind === 'search' && (
                  <>
                    <Text
                      className="help"
                      css={{
                        fontSize: '13px',
                        color: '{colors.fg.muted}',
                        lineHeight: '20px',
                        margin: '8px 0',
                      }}
                    >
                      Saved searches contain filter data. They grant no file permissions or
                      scheduled actions.
                    </Text>
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
                    <Text>{selectionCount.toLocaleString()} photos selected in the library.</Text>
                    <Box
                      className="catalog-actions"
                      css={{ display: 'flex', flexWrap: 'wrap', gap: '10px', margin: '12px 0' }}
                    >
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
                    </Box>
                  </>
                )}
              </Box>
            )}
            {loaded && !collections.length && !busy && (
              <Text>No albums or saved searches yet.</Text>
            )}
            <details>
              <summary>About albums &amp; saved searches</summary>
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                An album groups photos without moving them. A saved search finds photos using saved
                filters.
              </Text>
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                Up to 200 collections, including archived entries. An archived album keeps its
                photos for restoration.
              </Text>
            </details>
          </chakra.section>
          <chakra.section aria-labelledby="categories-title">
            <Heading as="h3" unstyled id="categories-title" tabIndex={-1}>
              Tag categories
            </Heading>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Group related tags under a parent. Choosing a parent also includes its child tags.
            </Text>
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Category to arrange</span>
                </Field.Label>
                <NativeSelect
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
                </NativeSelect>
              </label>
            </Field.Root>
            <chakra.form
              className="catalog-form"
              css={{
                display: 'flex',
                flexWrap: 'wrap',
                gap: '12px',
                alignItems: 'end',
                margin: '12px 0',
                '& label': { flex: '1', minWidth: '170px' },
                '& button': { marginBottom: '10px' },
                '@media (max-width: 760px)': { display: 'grid' },
              }}
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
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Tag label</span>
                  </Field.Label>
                  <Input
                    required
                    maxLength={128}
                    readOnly={Boolean(editingCategory)}
                    value={categoryName}
                    onChange={(e) => setCategoryName(e.target.value)}
                  />
                </label>
              </Field.Root>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Parent category</span>
                  </Field.Label>
                  <NativeSelect value={parent} onChange={(e) => setParent(e.target.value)}>
                    <option value="">No parent</option>
                    {categories
                      .filter((node) => node.id !== categoryId)
                      .map((node) => (
                        <option key={node.id} value={node.id}>
                          {node.label}
                        </option>
                      ))}
                  </NativeSelect>
                </label>
              </Field.Root>
              <Button type="submit">
                {editingCategory ? 'Save category parent' : 'Create tag category'}
              </Button>
            </chakra.form>
            <List.Root
              unstyled
              className="catalog-categories"
              css={{
                listStyle: 'none',
                padding: '0',
                display: 'grid',
                gap: '8px',
                '& li': {
                  display: 'flex',
                  justifyContent: 'space-between',
                  gap: '12px',
                  alignItems: 'center',
                },
                '& li > span': { minWidth: '0', overflowWrap: 'anywhere' },
                '& li > button': { flexShrink: '0' },
              }}
            >
              {categories.map((node) => (
                <List.Item unstyled key={node.id}>
                  <Text as="span">
                    {node.parentId
                      ? `${categories.find((parent) => parent.id === node.parentId)?.label ?? 'Unavailable parent'} → `
                      : ''}
                    <bdi>{node.label}</bdi>
                  </Text>
                  <Button
                    aria-label={`View category ${node.label}`}
                    onClick={() => {
                      onBrowse({ ...query, tag: node.label, tagDescendants: true });
                      onClose();
                    }}
                  >
                    View category
                  </Button>
                </List.Item>
              ))}
            </List.Root>
            <details>
              <summary>Tag matching &amp; limits</summary>
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                Tags match exactly: case, Unicode and labels containing slashes remain distinct.
              </Text>
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                Up to 512 categories and sixteen levels. Renaming/merging tag labels and sidecar
                export require their separate reviewed workflows.
              </Text>
            </details>
          </chakra.section>
          <TagTools
            onChanged={() => change(async () => {}, 'Tag metadata refreshed.')}
            onBusy={setBusy}
          />
        </chakra.fieldset>
        <MetadataExportPanel
          selection={selection}
          selectionCount={selectionCount}
          onBusy={setBusy}
        />
      </Box>
    </chakra.dialog>
  );
}
