import { mapFilterLabel } from '@/utils/map-filter-label';
import { Field, Box, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import type { AssetQuery, FolderEntry, LibraryStatus } from '../../bindings';
import type { Section } from '../../library/model';
import { SortControl } from '@/components/SortControl';
export function QueryFields({
  query,
  filter,
  status,
}: {
  query: AssetQuery;
  filter: (patch: Partial<AssetQuery>) => void;
  status: LibraryStatus;
}) {
  return (
    <Box className="panel-query-fields">
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Library</span>
          </Field.Label>
          <NativeSelect
            value={query.rootId ?? ''}
            onChange={(event) => filter({ rootId: event.target.value || null, folder: '' })}
          >
            <option value="">All libraries</option>
            {status.roots.map((root) => (
              <option value={root.id} key={root.id}>
                {root.label} · {root.fileCount.toLocaleString()}
              </option>
            ))}
          </NativeSelect>
        </label>
      </Field.Root>
      <SortControl value={query.sort} onChange={(sort) => filter({ sort })} />

      <chakra.section className="panel-criteria" css={{ display: 'grid', gap: '16px' }}>
        <Box
          className="form-grid"
          css={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
            gap: '16px',
            width: '100%',
            '@media (max-width: 600px)': { gridTemplateColumns: '1fr' },
          }}
        >
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>From date</span>
              </Field.Label>
              <Input
                type="date"
                value={query.fromDate}
                onChange={(event) => filter({ fromDate: event.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Through date</span>
              </Field.Label>
              <Input
                type="date"
                value={query.throughDate}
                onChange={(event) => filter({ throughDate: event.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Minimum bytes</span>
              </Field.Label>
              <Input
                inputMode="numeric"
                value={query.minBytes}
                onChange={(event) => filter({ minBytes: event.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Maximum bytes</span>
              </Field.Label>
              <Input
                inputMode="numeric"
                value={query.maxBytes}
                onChange={(event) => filter({ maxBytes: event.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Extension</span>
              </Field.Label>
              <Input
                value={query.extension}
                placeholder="jpg"
                onChange={(event) => filter({ extension: event.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Tag</span>
              </Field.Label>
              <Input value={query.tag} onChange={(event) => filter({ tag: event.target.value })} />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Minimum rating</span>
              </Field.Label>
              <NativeSelect
                value={query.minRating ?? ''}
                onChange={(event) =>
                  filter({
                    minRating: event.target.value === '' ? null : Number(event.target.value),
                  })
                }
              >
                <option value="">Any rating</option>
                {[1, 2, 3, 4, 5].map((value) => (
                  <option key={value} value={value}>
                    {value} / 5 or higher
                  </option>
                ))}
              </NativeSelect>
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Review flag</span>
              </Field.Label>
              <NativeSelect
                value={
                  query.rejected === null || query.rejected === undefined
                    ? 'any'
                    : query.rejected
                      ? 'rejected'
                      : 'not_rejected'
                }
                onChange={(event) =>
                  filter({
                    rejected:
                      event.target.value === 'any' ? null : event.target.value === 'rejected',
                  })
                }
              >
                <option value="any">All photos</option>
                <option value="rejected">Rejected only</option>
                <option value="not_rejected">Not rejected</option>
              </NativeSelect>
            </label>
          </Field.Root>
        </Box>
        <chakra.label
          className="check-row"
          css={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '10px',
            margin: '12px 0',
            '& label': { flexDirection: 'row', alignItems: 'center' },
          }}
        >
          <input
            type="checkbox"
            checked={query.favoritesOnly}
            onChange={(event) => filter({ favoritesOnly: event.target.checked })}
          />{' '}
          Favorites only
        </chakra.label>
        <chakra.label
          className="check-row"
          css={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '10px',
            margin: '12px 0',
            '& label': { flexDirection: 'row', alignItems: 'center' },
          }}
        >
          <input
            type="checkbox"
            checked={query.tagDescendants ?? false}
            onChange={(event) => filter({ tagDescendants: event.target.checked })}
          />
          Include tag descendants
        </chakra.label>
        <chakra.label
          className="check-row"
          css={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '10px',
            margin: '12px 0',
            '& label': { flexDirection: 'row', alignItems: 'center' },
          }}
        >
          <input
            type="checkbox"
            checked={query.includeMissing}
            onChange={(event) => filter({ includeMissing: event.target.checked })}
          />{' '}
          Include missing files
        </chakra.label>
        <chakra.label
          className="check-row"
          css={{
            display: 'flex',
            flexDirection: 'row',
            alignItems: 'center',
            gap: '10px',
            margin: '12px 0',
            '& label': { flexDirection: 'row', alignItems: 'center' },
          }}
          title="Visual-similarity analysis is not available yet. No groups are inferred from filenames or cached previews."
        >
          <input type="checkbox" disabled />
          Similar groups · unavailable
        </chakra.label>
      </chakra.section>
    </Box>
  );
}

export function LibraryFacets({
  section,
  query,
  folders,
  tags,
  onFilter,
}: {
  section: Section;
  query: AssetQuery;
  folders: FolderEntry[];
  tags: string[];
  onFilter: (patch: Partial<AssetQuery>) => void;
}) {
  return (
    <>
      {section === 'Folders' && (
        <Box
          className="folder-strip"
          css={{
            '& h2': { fontSize: '17px', letterSpacing: '0', width: '100%', marginBottom: '4px' },
            display: 'flex',
            flexWrap: 'wrap',
            gap: '8px',
            padding: '16px 0',
            borderBottom: '1px solid {colors.border}',
            maxHeight: '160px',
            overflow: 'auto',
            '& p': { fontSize: '12px', color: '{colors.fg.muted}' },
            '& button': { maxWidth: '100%', overflowWrap: 'anywhere' },
          }}
        >
          <Heading as="h2" unstyled>
            Folders
          </Heading>
          {!query.rootId ? (
            <Text>Select a library to browse its folders.</Text>
          ) : (
            <>
              <Button
                type="button"
                aria-pressed={!query.folder}
                onClick={() => onFilter({ folder: '' })}
              >
                All folders
              </Button>
              {folders.map((folder) => (
                <Button
                  type="button"
                  key={folder.path}
                  aria-pressed={query.folder === folder.path}
                  onClick={() => onFilter({ folder: folder.path })}
                >
                  <bdi>{folder.path || 'Library folder'}</bdi> <small>{folder.count}</small>
                </Button>
              ))}
            </>
          )}
        </Box>
      )}
      {section === 'Tags' && (
        <Box
          className="folder-strip"
          css={{
            '& h2': { fontSize: '17px', letterSpacing: '0', width: '100%', marginBottom: '4px' },
            display: 'flex',
            flexWrap: 'wrap',
            gap: '8px',
            padding: '16px 0',
            borderBottom: '1px solid {colors.border}',
            maxHeight: '160px',
            overflow: 'auto',
            '& p': { fontSize: '12px', color: '{colors.fg.muted}' },
            '& button': { maxWidth: '100%', overflowWrap: 'anywhere' },
          }}
        >
          <Heading as="h2" unstyled>
            Tags
          </Heading>
          {tags.length ? (
            tags.map((tag) => (
              <Button
                type="button"
                key={tag}
                aria-pressed={query.tag === tag}
                onClick={() => onFilter({ tag: query.tag === tag ? '' : tag })}
              >
                {tag}
              </Button>
            ))
          ) : (
            <Text>Add tags in Photo details to organize your library.</Text>
          )}
        </Box>
      )}
    </>
  );
}

export function RootAvailability({
  status,
  rootId,
  onSettings,
  onRelink,
}: {
  status: LibraryStatus;
  rootId: string | null;
  onSettings: (id: string) => void;
  onRelink: (id: string) => Promise<void>;
}) {
  const root = status.roots.find((item) => item.id === rootId);
  return (
    <>
      {root && (
        <Box
          className="root-status"
          css={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-start',
            gap: '12px',
            fontSize: '12px',
            color: '{colors.fg.muted}',
            marginTop: '14px',
            flexWrap: 'wrap',
            '& span': { marginRight: 'auto' },
          }}
        >
          <Text as="span">
            <bdi>{root.label}</bdi> · {root.availability.replaceAll('_', ' ')} ·{' '}
            {root.mode === 'referenced' ? 'read only' : 'managed'}
          </Text>
          <Button type="button" onClick={() => onSettings(root.id)}>
            Scan settings
          </Button>
          {root.availability !== 'online' && root.availability !== 'scanning' && (
            <Button type="button" onClick={() => void onRelink(root.id)}>
              Relink folder
            </Button>
          )}
        </Box>
      )}
      {status.roots.some((root) =>
        ['offline', 'inaccessible', 'needs_relink'].includes(root.availability),
      ) && (
        <Text
          className="availability-note"
          css={{
            background: '{colors.bg.subtle}',
            color: '{colors.fg.secondary}',
            padding: '12px',
            fontSize: '12px',
            lineHeight: '1.5',
          }}
        >
          Some originals are unavailable. Catalog details and cached previews remain available.
          Select a library to reconnect it.
        </Text>
      )}
    </>
  );
}

export function ActiveFilters({
  query,
  filter,
}: {
  query: AssetQuery;
  filter: (patch: Partial<AssetQuery>) => void;
}) {
  const mapScope = mapFilterLabel(query);
  const fields = [
    'name',
    'folder',
    'tag',
    'extension',
    'fromDate',
    'throughDate',
    'minBytes',
    'maxBytes',
  ] as const;
  const names = {
    name: 'Filename or date',
    folder: 'Folder',
    tag: 'Tag',
    extension: 'Extension',
    fromDate: 'From',
    throughDate: 'Through',
    minBytes: 'Minimum bytes',
    maxBytes: 'Maximum bytes',
  };
  return (
    <Box
      className="active-filters"
      css={{
        display: 'flex',
        flexWrap: 'wrap',
        gap: '4px',
        '&:empty': { display: 'none' },
        '& button': {
          maxWidth: '200px',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
          fontSize: '11px',
          padding: '3px 7px',
          minHeight: '28px',
        },
      }}
      aria-label="Active filters"
    >
      {fields
        .filter((key) => query[key])
        .map((key) => (
          <Button
            type="button"
            key={key}
            aria-label={`Remove ${names[key]} filter`}
            onClick={() => filter({ [key]: '' })}
          >
            {names[key]}: <bdi>{query[key]}</bdi> ×
          </Button>
        ))}
      {mapScope && (
        <Button
          type="button"
          aria-label={`Remove ${mapScope} filter`}
          onClick={() => filter({ mapBounds: null, mapCell: null, location: null })}
        >
          {mapScope} ×
        </Button>
      )}
      {query.favoritesOnly && (
        <Button type="button" onClick={() => filter({ favoritesOnly: false })}>
          Favorites ×
        </Button>
      )}
      {query.placeId && (
        <Button type="button" onClick={() => filter({ placeId: null })}>
          Place ×
        </Button>
      )}
      {query.albumId && (
        <Button type="button" onClick={() => filter({ albumId: null })}>
          Album ×
        </Button>
      )}
      {query.minRating !== null && query.minRating !== undefined && (
        <Button type="button" onClick={() => filter({ minRating: null })}>
          Rating ≥ {query.minRating} ×
        </Button>
      )}
      {query.rejected !== null && query.rejected !== undefined && (
        <Button type="button" onClick={() => filter({ rejected: null })}>
          {query.rejected ? 'Rejected' : 'Not rejected'} ×
        </Button>
      )}
    </Box>
  );
}
