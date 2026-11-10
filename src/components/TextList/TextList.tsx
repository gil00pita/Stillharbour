import { Box, Text, chakra, Button } from '@chakra-ui/react';
import {
  usePhotoAlbumMenu,
  usePhotoAlbumKeys,
  usePhotoSelection,
} from '@/components/PhotoAlbumActions';
import { useEffect, useRef, type KeyboardEvent } from 'react';
import type { AssetSummary } from '../../bindings';
import { displayDate, fileSize } from '../../library/model';
import { SortControl } from '@/components/SortControl';

import {
  DEFAULT_TEXT_COLUMNS,
  TEXT_COLUMNS,
  textListLayout,
  type TextColumn,
} from '../../library/textColumns';

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
    <Box
      className="text-list-controls"
      css={{
        display: 'flex',
        alignItems: 'end',
        justifyContent: 'space-between',
        flexWrap: 'wrap',
        gap: '12px',
        marginBottom: '12px',
        flexShrink: '0',
        '& > label': { flexDirection: 'row', alignItems: 'center' },
      }}
    >
      <SortControl value={sort} onChange={onSort} label="Text list sort" disabled={busy} />
      <chakra.details
        ref={menu}
        className="text-list-columns"
        css={{
          position: 'relative',
          '& summary': {
            padding: '8px 12px',
            border: '1px solid {colors.border}',
            borderRadius: 'var(--control-radius)',
          },
          '& fieldset': {
            position: 'absolute',
            top: '100%',
            right: '0',
            zIndex: '5',
            width: '240px',
            maxWidth: 'calc(100vw - 48px)',
            maxHeight: 'min(350px, 55vh)',
            overflowY: 'auto',
            margin: '6px 0 0',
            padding: '12px',
            background: '{colors.bg.panel}',
            border: '1px solid {colors.border}',
            borderRadius: 'var(--control-radius)',
          },
          '& legend': {
            float: 'left',
            width: '100%',
            marginBottom: '8px',
            fontSize: '12px',
            fontWeight: '600',
          },
          '& .check-row': { clear: 'both', margin: '0', padding: '6px 0' },
          '& button': { marginTop: '8px' },
        }}
      >
        <summary>Columns</summary>
        <chakra.fieldset>
          <chakra.legend>Visible columns</chakra.legend>
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
            <input type="checkbox" checked disabled /> Name (always shown)
          </chakra.label>
          {TEXT_COLUMNS.map((column) => (
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
              key={column.id}
            >
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
            </chakra.label>
          ))}
          <Button unstyled type="button" onClick={() => onColumns([...DEFAULT_TEXT_COLUMNS])}>
            Reset columns
          </Button>
        </chakra.fieldset>
      </chakra.details>
    </Box>
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
    <Box
      className="text-list-head"
      css={{
        display: 'grid',
        gridTemplateColumns: 'var(--text-list-grid)',
        alignItems: 'center',
        fontSize: '12px',
        borderBottom: '1px solid {colors.border}',
        minHeight: '40px',
        color: '{colors.fg.muted}',
        overflowY: 'hidden',
        scrollbarGutter: 'stable',
        flexShrink: '0',
        '& > span': { minWidth: '0', paddingRight: '12px' },
        '& button:not(.primary):not(.asset-open):not(.chakra-button)': {
          display: 'flex',
          alignItems: 'center',
          gap: '6px',
          border: '0',
          padding: '0',
          background: 'transparent',
          color: 'inherit',
          textAlign: 'left',
          minHeight: '36px',
        },
        '& [aria-sort]': { color: '{colors.brand.solid}' },
      }}
      role="row"
    >
      <Text as="span" role="columnheader" aria-label="Selection" />
      {fields.map((column) => {
        const active = sort === column.sort;
        return (
          <Text
            as="span"
            role="columnheader"
            key={column.id}
            aria-sort={active ? (sort === 'name' ? 'ascending' : 'descending') : undefined}
          >
            {column.sort && onSort ? (
              <Button
                unstyled
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
              </Button>
            ) : (
              column.label
            )}
          </Text>
        );
      })}
    </Box>
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
  const contextMenu = usePhotoAlbumMenu(asset);
  const contextKeys = usePhotoAlbumKeys(asset);
  const selection = usePhotoSelection();
  const selecting = selection?.active ?? selectionActive;
  return (
    <Box
      onContextMenu={contextMenu}
      onKeyDown={contextKeys}
      role="row"
      tabIndex={-1}
      className={`asset text-list-row ${selected ? 'is-selected' : ''}`}
      css={{
        position: 'relative',
        borderRadius: '5px',
        minWidth: '0',
        flexShrink: '0',
        '&.is-selected': {
          outline: '2px solid {colors.brand.solid}',
          outlineOffset: '-2px',
          background: '{colors.brand.selected}',
        },
        '&.is-active': { boxShadow: 'inset 0 -2px 0 {colors.brand.solid}' },
        display: 'grid',
        gridTemplateColumns: 'var(--text-list-grid)',
        alignItems: 'center',
        fontSize: '12px',
        borderBottom: '1px solid {colors.border}',
        '& > span': { minWidth: '0', paddingRight: '12px' },
        '& > span:not(.text-list-selection)': {
          overflow: 'hidden',
          whiteSpace: 'nowrap',
          textOverflow: 'ellipsis',
        },
        '& .text-list-selection': { padding: '0' },
        '& .text-list-name': { display: 'flex', alignItems: 'center', gap: '8px' },
        '& .text-list-name .asset-open': { minWidth: '0', flex: '1' },
      }}
      data-asset-id={asset.id}
      data-date={asset.date}
    >
      <Text as="span" role="cell" className="text-list-selection">
        {(!selection || selecting) && (
          <chakra.input
            className="asset-check"
            css={{ position: 'absolute', top: '9px', left: '9px', zIndex: '1', margin: '0' }}
            type="checkbox"
            aria-label={`Select ${asset.name}`}
            checked={selected}
            disabled={frozen}
            onChange={(event) =>
              onSelect(asset.id, Boolean((event.nativeEvent as MouseEvent).shiftKey))
            }
            onKeyDown={onNavigate}
          />
        )}
      </Text>
      <Text as="span" role="cell" className="text-list-name">
        <Button
          unstyled
          type="submit"
          className="asset-open"
          css={{
            border: '0',
            background: 'transparent',
            padding: '0',
            color: 'inherit',
            width: '100%',
            textAlign: 'left',
            fontSize: '12px',
            height: '100%',
            display: 'flex',
            flexDirection: 'column',
          }}
          aria-label={`Open ${asset.name}`}
          title={asset.name}
          onKeyDown={onNavigate}
          onClick={(event) => {
            if (event.shiftKey || selecting) {
              if (!frozen) onSelect(asset.id, event.shiftKey);
            } else onOpen(asset.id, event.currentTarget);
          }}
        >
          <bdi>{asset.name}</bdi>
        </Button>
      </Text>
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
          <Text
            as="span"
            role="cell"
            key={column.id}
            data-column={column.id}
            title={rawDate || value}
          >
            <bdi>{value}</bdi>
          </Text>
        );
      })}
    </Box>
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
