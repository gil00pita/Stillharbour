import { useEffect, useRef, useState } from 'react';
import type { AssetQuery, CatalogCollection, LibraryRoot } from '../../bindings';
import { Button } from '@/components/Button';
import { libraryData } from '../../ipc';

const labels: Record<keyof AssetQuery, string> = {
  placeId: 'Place',
  rootId: 'Library',
  directoryId: 'Indexed folder',
  folder: 'Folder path',
  name: 'Filename or date',
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
      {(Object.keys(labels) as Array<keyof AssetQuery>)
        .map((key) => [key, query[key]] as const)
        .filter(([, value]) => value !== null && value !== undefined && value !== '')
        .map(([key, value]) => {
          let display = String(value);
          if (key === 'mapBounds' && query.mapBounds) {
            const { west, east, south, north } = query.mapBounds;
            display = `Longitude ${west}° to ${east}°; latitude ${south}° to ${north}°`;
          }
          if (key === 'mapCell' && query.mapCell) {
            const { zoom, x, y } = query.mapCell;
            display = `Selected region: zoom ${zoom}, column ${x}, row ${y}`;
          }
          if (key === 'location')
            display =
              value === 'known'
                ? 'With usable location'
                : value === 'unavailable'
                  ? 'Without usable location'
                  : 'Any location';
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
              <dd>{key === 'placeId' ? <PlaceLabel id={display} /> : <bdi>{display}</bdi>}</dd>
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

function PlaceLabel({ id }: { id: string }) {
  const [result, setResult] = useState<{ id: string; label: string } | null>(null);
  useEffect(() => {
    let active = true;
    void libraryData({ kind: 'places', text: '', selected_id: id }, 'places')
      .then((value) => {
        if (active) setResult({ id, label: value.selected?.label ?? 'Place unavailable' });
      })
      .catch(() => {
        if (active) setResult({ id, label: 'Place details could not be loaded' });
      });
    return () => {
      active = false;
    };
  }, [id]);
  return <bdi>{result?.id === id ? result.label : 'Loading place…'}</bdi>;
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
  const review = useRef<HTMLElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const opened = useRef(false);
  useEffect(() => {
    if (proposed) {
      opened.current = true;
      review.current?.focus({ preventScroll: true });
      review.current?.scrollIntoView?.({ block: 'start' });
    } else if (opened.current) trigger.current?.focus();
  }, [proposed]);
  if (!proposed)
    return (
      <Button ref={trigger} onClick={() => setProposed(structuredClone(query))}>
        Replace saved filters with current filters
      </Button>
    );
  return (
    <section
      ref={review}
      tabIndex={-1}
      className="catalog-search-review"
      aria-label="Review saved search filters"
    >
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
