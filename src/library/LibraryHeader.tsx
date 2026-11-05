import { Button } from '../components/ui/controls';
import type { ReactNode } from 'react';
import type { Section } from './model';
import { PlaceSearch } from '../components/PlaceSearch';
import type { PlaceSummary } from '../bindings';
export function LibraryHeader({
  jobsControl,
  navigationControl,
  section,
  value,
  onSearch,
  placeId,
  onPlace,
  busy,
  onFilter,
  filterOpen,
  activeCount,
  onPlans,
  onCatalog,
}: {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  section: Section;
  value: string;
  onSearch: (value: string) => void;
  placeId?: string | null | undefined;
  onPlace: (place: PlaceSummary | null) => void;
  busy: boolean;
  onFilter: () => void;
  filterOpen: boolean;
  activeCount: number;
  onPlans?: () => void;
  onCatalog?: () => void;
}) {
  return (
    <header className="toolbar photo-toolbar">
      {navigationControl}
      <h1>{section === 'Tags' ? 'Albums / Tags' : section}</h1>
      <PlaceSearch
        value={value}
        onSearch={onSearch}
        selectedId={placeId}
        onChange={onPlace}
        disabled={busy}
      />
      <Button
        onClick={onFilter}
        disabled={busy}
        aria-label={activeCount ? `Filter (${activeCount} active)` : 'Filter'}
        aria-haspopup="dialog"
        aria-expanded={filterOpen}
        aria-controls="library-filter-panel"
      >
        Filter{activeCount > 0 && <span className="filter-count">{activeCount}</span>}
      </Button>
      {jobsControl}
      {onCatalog && (
        <Button disabled={busy} onClick={onCatalog}>
          Catalog
        </Button>
      )}
      {onPlans && (
        <Button onClick={onPlans} disabled={busy}>
          Saved plans
        </Button>
      )}
    </header>
  );
}
