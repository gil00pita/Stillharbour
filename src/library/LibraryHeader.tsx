import { Button, Input } from '../components/ui/controls';
import { IconButton } from '@chakra-ui/react';
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
      <label className="fixed-search">
        <span className="screen-reader-status">Search filenames or dates</span>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="10.5" cy="10.5" r="6.5" stroke="currentColor" strokeWidth="1.5" />
          <path d="m16 16 4.5 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
        <Input
          type="search"
          aria-label="Search filenames or dates"
          placeholder="Search filenames or dates…"
          title="Search filenames or a day: DD-MM-YYYY or YYYY-MM-DD"
          value={value}
          maxLength={256}
          disabled={busy}
          onChange={(event) => onSearch(event.target.value)}
        />
      </label>
      <PlaceSearch selectedId={placeId} onChange={onPlace} disabled={busy} />
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
      {onCatalog && (
        <IconButton
          type="button"
          variant="outline"
          className="toolbar-secondary"
          aria-label="Catalog"
          title="Catalog"
          disabled={busy}
          onClick={onCatalog}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M4 5h16v15H4ZM8 3v4m8-4v4M4 10h16M8 14h3m-3 3h7"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          <span className="toolbar-action-label">Catalog</span>
        </IconButton>
      )}
      {onPlans && (
        <IconButton
          type="button"
          variant="outline"
          className="toolbar-secondary"
          aria-label="Saved plans"
          title="Saved plans"
          onClick={onPlans}
          disabled={busy}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M6 3h12v18l-6-4-6 4Z"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          <span className="toolbar-action-label">Saved plans</span>
        </IconButton>
      )}
    </header>
  );
}
