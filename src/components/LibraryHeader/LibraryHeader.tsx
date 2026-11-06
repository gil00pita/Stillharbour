import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import type { ReactNode } from 'react';
import type { Section } from '../../library/model';
import { PlaceSearch } from '@/components/PlaceSearch';
import type { PlaceSummary } from '../../bindings';
export function LibraryHeader({
  jobsControl,
  navigationControl,
  section,
  value,
  onSearch,
  placeId,
  mapScope,
  onPlace,
  busy,
  onFilter,
  filterOpen,
  activeCount,
  selecting = false,
  onSelect,
}: {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  section: Section;
  value: string;
  onSearch: (value: string) => void;
  placeId?: string | null | undefined;
  mapScope?: { label: string; onClear: () => void } | undefined;
  onPlace: (place: PlaceSummary | null) => void;
  busy: boolean;
  onFilter: () => void;
  filterOpen: boolean;
  activeCount: number;
  selecting?: boolean;
  onSelect?: () => void;
}) {
  return (
    <PageHeader>
      <header className="toolbar photo-toolbar">
        {navigationControl}
        <h1>{section === 'Tags' ? 'Albums / Tags' : section}</h1>
        <PlaceSearch
          value={value}
          onSearch={onSearch}
          selectedId={placeId}
          onChange={onPlace}
          disabled={busy}
          filterScope={mapScope}
        />
        <Button
          onClick={onFilter}
          disabled={busy}
          aria-label={activeCount ? `Filter (${activeCount} active)` : 'Filter'}
          aria-haspopup="dialog"
          aria-expanded={filterOpen}
          aria-controls="library-filter-panel"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M4 7h16M4 17h16M9 4v6m6 4v6"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          Filter{activeCount > 0 && <span className="filter-count">{activeCount}</span>}
        </Button>
        {jobsControl}
        {onSelect && (
          <Button type="button" aria-pressed={selecting} disabled={busy} onClick={onSelect}>
            {selecting ? 'Done selecting' : 'Select'}
          </Button>
        )}
      </header>
    </PageHeader>
  );
}
