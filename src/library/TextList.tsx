import { useEffect, useRef, type KeyboardEvent } from 'react';
import type { AssetSummary } from '../bindings';
import { displayDate, fileSize } from './model';
import { SortControl } from './SortControl';

import { DEFAULT_TEXT_COLUMNS, TEXT_COLUMNS, textListLayout, type TextColumn } from './textColumns';

export function TextListControls({
  columns,
  onColumns,
  sort,
  onSort,
  busy,
}: {
  columns: readonly TextColumn[];
  onColumns: (columns: TextColumn[]) => void;
  sort: string;
  onSort: (sort: string) => void;
  busy: boolean;
}) {
  const menu = useRef<HTMLDetailsElement>(null);
  useEffect(() => {
    function close(event: Event) {
      const element = menu.current;
      if (!element?.open) return;
      if (event instanceof globalThis.KeyboardEvent) {
        if (event.key !== 'Escape') return;
        event.preventDefault();
        element.open = false;
        element.querySelector('summary')?.focus();
      } else if (event.target instanceof Node && !element.contains(event.target)) {
        element.open = false;
      }
    }
    document.addEventListener('pointerdown', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('pointerdown', close);
      document.removeEventListener('keydown', close);
    };
  }, []);
  return (
    <div className="text-list-controls">
      <SortControl value={sort} onChange={onSort} label="Text list sort" disabled={busy} />
      <details ref={menu} className="text-list-columns">
        <summary>Columns</summary>
        <fieldset>
          <legend>Visible columns</legend>
          <label className="check-row">
            <input type="checkbox" checked disabled /> Name (always shown)
          </label>
          {TEXT_COLUMNS.map((column) => (
            <label className="check-row" key={column.id}>
              <input
                type="checkbox"
                name={`text-column-${column.id}`}
                checked={columns.includes(column.id)}
                onChange={(event) =>
                  onColumns(
                    event.target.checked
                      ? [...columns, column.id]
                      : columns.filter((id) => id !== column.id),
                  )
                }
              />
              {column.label}
            </label>
          ))}
          <button type="button" onClick={() => onColumns([...DEFAULT_TEXT_COLUMNS])}>
            Reset columns
          </button>
        </fieldset>
      </details>
    </div>
  );
}

export function TextListHeader({
  columns,
  sort,
  onSort,
  busy,
}: {
  columns: readonly TextColumn[];
  sort?: string | undefined;
  onSort?: ((sort: string) => void) | undefined;
  busy: boolean;
}) {
  const fields = [{ id: 'name', label: 'Name', sort: 'name' }, ...textListLayout(columns).visible];
  return (
    <div className="text-list-head" role="row">
      <span role="columnheader" aria-label="Selection" />
      {fields.map((column) => {
        const active = sort === column.sort;
        return (
          <span
            role="columnheader"
            key={column.id}
            aria-sort={active ? (sort === 'name' ? 'ascending' : 'descending') : undefined}
          >
            {column.sort && onSort ? (
              <button
                type="button"
                data-sort={column.sort}
                disabled={busy}
                title={
                  column.sort === 'name'
                    ? 'Sort by filename, A to Z'
                    : column.sort === 'size'
                      ? 'Sort by size, largest first'
                      : `Sort by ${column.label}, newest first`
                }
                onClick={() => {
                  if (!active && column.sort) onSort(column.sort);
                }}
              >
                {column.label}
                {active && (
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                    <path
                      d={sort === 'name' ? 'm4 10 4-4 4 4' : 'm4 6 4 4 4-4'}
                      stroke="currentColor"
                      strokeWidth="1.5"
                    />
                  </svg>
                )}
              </button>
            ) : (
              column.label
            )}
          </span>
        );
      })}
    </div>
  );
}

export function TextListRow({
  asset,
  columns,
  selected,
  frozen,
  selectionActive,
  offline,
  onSelect,
  onOpen,
  onNavigate,
}: {
  asset: AssetSummary;
  columns: readonly TextColumn[];
  selected: boolean;
  frozen: boolean;
  selectionActive: boolean;
  offline: boolean;
  onSelect: (id: string, range: boolean) => void;
  onOpen: (id: string, opener: HTMLButtonElement) => void;
  onNavigate: (event: KeyboardEvent<HTMLElement>) => void;
}) {
  return (
    <div
      role="row"
      className={`asset text-list-row ${selected ? 'is-selected' : ''}`}
      data-asset-id={asset.id}
      data-date={asset.date}
    >
      <span role="cell" className="text-list-selection">
        <input
          className="asset-check"
          type="checkbox"
          aria-label={`Select ${asset.name}`}
          checked={selected}
          disabled={frozen}
          onChange={(event) =>
            onSelect(asset.id, Boolean((event.nativeEvent as MouseEvent).shiftKey))
          }
          onKeyDown={onNavigate}
        />
      </span>
      <span role="cell">
        <button
          className="asset-open"
          aria-label={`Open ${asset.name}`}
          title={asset.name}
          onKeyDown={onNavigate}
          onClick={(event) => {
            if (event.shiftKey || selectionActive) {
              if (!frozen) onSelect(asset.id, event.shiftKey);
            } else onOpen(asset.id, event.currentTarget);
          }}
        >
          <bdi>{asset.name}</bdi>
        </button>
      </span>
      {textListLayout(columns).visible.map((column) => {
        const value = textColumnValue(column.id, asset, offline);
        const rawDate =
          column.id === 'modified'
            ? asset.modifiedAt
            : column.id === 'capture'
              ? asset.date
              : column.id === 'added'
                ? asset.addedAt
                : null;
        return (
          <span role="cell" key={column.id} data-column={column.id} title={rawDate || value}>
            <bdi>{value}</bdi>
          </span>
        );
      })}
    </div>
  );
}

function textColumnValue(column: TextColumn, asset: AssetSummary, offline: boolean): string {
  switch (column) {
    case 'modified':
      return asset.modifiedAt ? displayDate(asset.modifiedAt) : 'Unknown';
    case 'size':
      return fileSize(asset.bytes);
    case 'dimensions':
      return asset.width && asset.height ? `${asset.width} × ${asset.height}` : 'Unknown';
    case 'folder':
      return asset.folder || 'Library folder';
    case 'status':
      return offline
        ? 'Offline original'
        : asset.presence === 'present'
          ? asset.metadataState
          : asset.presence;
    case 'capture':
      return asset.date ? displayDate(asset.date) : 'Undated';
    case 'source':
      return asset.dateSource.replaceAll('_', ' ');
    case 'added':
      return asset.addedAt ? displayDate(asset.addedAt) : 'Unknown';
    case 'format':
      return asset.detectedFormat || asset.extension || 'Unknown';
    case 'camera':
      return asset.camera || 'Unknown';
    case 'rating':
      return `${asset.rating} / 5`;
    case 'tags':
      return asset.tags.join(', ') || 'None';
  }
}
