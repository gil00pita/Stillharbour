import type { ReactNode } from 'react';
import type { Section } from './model';
export function LibraryHeader({
  jobsControl,
  navigationControl,
  section,
  value,
  onSearch,
  busy,
  onFilter,
  filterOpen,
  activeCount,
  onPlans,
}: {
  jobsControl?: ReactNode;
  navigationControl?: ReactNode;
  section: Section;
  value: string;
  onSearch: (value: string) => void;
  busy: boolean;
  onFilter: () => void;
  filterOpen: boolean;
  activeCount: number;
  onPlans?: () => void;
}) {
  return (
    <header className="toolbar photo-toolbar">
      {navigationControl}
      <h1>{section === 'Tags' ? 'Albums / Tags' : section}</h1>
      <label className="fixed-search">
        <span className="screen-reader-status">Search filenames</span>
        <input
          type="search"
          aria-label="Search filenames"
          placeholder="Search filenames…"
          value={value}
          maxLength={256}
          disabled={busy}
          onChange={(event) => onSearch(event.target.value)}
        />
      </label>
      <button
        onClick={onFilter}
        disabled={busy}
        aria-label={activeCount ? `Filter (${activeCount} active)` : 'Filter'}
        aria-haspopup="dialog"
        aria-expanded={filterOpen}
        aria-controls="library-filter-panel"
      >
        Filter{activeCount > 0 && <span className="filter-count">{activeCount}</span>}
      </button>
      {jobsControl}
      {onPlans && (
        <button onClick={onPlans} disabled={busy}>
          Saved plans
        </button>
      )}
    </header>
  );
}
