import { useState } from 'react';
import type { AssetQuery, CatalogCollection, LibraryRoot } from '../bindings';
import { Button } from '../components/ui/controls';

const labels: Record<keyof AssetQuery, string> = {
  rootId: 'Library',
  directoryId: 'Indexed folder',
  folder: 'Folder path',
  name: 'Filename',
  fromDate: 'From date',
  throughDate: 'Through date',
  minBytes: 'Minimum bytes',
  maxBytes: 'Maximum bytes',
  extension: 'File extension',
  favoritesOnly: 'Favorites only',
  tag: 'Tag',
  sort: 'Order',
  includeMissing: 'Include missing photos',
  albumId: 'Album',
  rejected: 'Rejected',
  minRating: 'Minimum rating',
  tagDescendants: 'Include child tags',
  mapCell: 'Map area',
  mapBounds: 'Map bounds',
  location: 'Location',
  deferPendingPreviews: 'Wait for previews',
};
function FilterSummary({
  query,
  roots,
  collections,
}: {
  query: AssetQuery;
  roots: LibraryRoot[];
  collections: CatalogCollection[];
}) {
  return (
    <dl className="catalog-filter-summary">
      {Object.entries(query)
        .filter(([, value]) => value !== null && value !== undefined && value !== '')
        .map(([key, value]) => {
          let display = typeof value === 'object' ? JSON.stringify(value) : String(value);
          if (typeof value === 'boolean') display = value ? 'Yes' : 'No';
          if (key === 'rootId')
            display =
              roots.find((root) => root.id === value)?.label ?? `Unavailable library (${display})`;
          if (key === 'albumId')
            display =
              collections.find((item) => item.id === value)?.name ??
              `Unavailable album (${display})`;
          if (key === 'sort')
            display =
              (
                {
                  capture: 'Capture date',
                  added: 'Recently added',
                  name: 'Filename',
                  modified: 'Modified date',
                  size: 'File size',
                } as Record<string, string>
              )[display] ?? display;
          return (
            <div key={key}>
              <dt>{labels[key as keyof AssetQuery] ?? key}</dt>
              <dd>
                <bdi>{display}</bdi>
              </dd>
            </div>
          );
        })}
      {!query.rootId && (
        <div>
          <dt>Library</dt>
          <dd>All libraries</dd>
        </div>
      )}
    </dl>
  );
}

export function SavedSearchReview({
  current,
  query,
  roots,
  collections,
  onApply,
}: {
  current: CatalogCollection;
  query: AssetQuery;
  roots: LibraryRoot[];
  collections: CatalogCollection[];
  onApply: (query: AssetQuery) => Promise<void>;
}) {
  const [proposed, setProposed] = useState<AssetQuery | null>(null);
  if (!proposed)
    return (
      <Button onClick={() => setProposed(structuredClone(query))}>
        Replace saved filters with current filters
      </Button>
    );
  return (
    <section className="catalog-search-review" aria-label="Review saved search filters">
      <h4>
        Replace filters for <bdi>{current.name}</bdi>?
      </h4>
      <p className="help">
        Review both sets of filters before replacing this saved search. Photos and other searches
        stay unchanged.
      </p>
      <div className="catalog-filter-comparison">
        <section aria-label="Saved filters">
          <h4>Saved filters</h4>
          <FilterSummary query={current.query!} roots={roots} collections={collections} />
        </section>
        <section aria-label="Replacement filters">
          <h4>Replacement filters</h4>
          <FilterSummary query={proposed} roots={roots} collections={collections} />
        </section>
      </div>
      <div className="catalog-actions">
        <Button onClick={() => setProposed(null)}>Keep saved filters</Button>
        <Button className="primary" onClick={() => void onApply(proposed)}>
          Replace saved search filters
        </Button>
      </div>
    </section>
  );
}
