import { Field, Popover, Portal, Stack } from '@chakra-ui/react';
import { useState } from 'react';
import type { AssetQuery, LibraryStatus } from '../../bindings';
import { Button, Input, Select } from '../ui/controls';

export function MapFilters({
  query,
  roots,
  unlocated,
  unavailable,
  onFilter,
  onLocation,
}: {
  query: AssetQuery;
  roots: LibraryStatus['roots'];
  unlocated: boolean;
  unavailable: number | null;
  onFilter: (patch: Partial<AssetQuery>) => void;
  onLocation: () => void;
}) {
  const [open, setOpen] = useState(false);
  const count = [query.rootId, query.fromDate, query.throughDate, unlocated].filter(Boolean).length;
  return (
    <Popover.Root
      open={open}
      onOpenChange={(event) => setOpen(event.open)}
      positioning={{ placement: 'bottom-end' }}
      lazyMount
      unmountOnExit
    >
      <Popover.Trigger asChild>
        <Button
          type="button"
          className="map-filter-trigger"
          aria-label={count ? `Map filters (${count} active)` : 'Map filters'}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M4 7h16M4 17h16M9 4v6m6 4v6"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          Filters{count > 0 && <span className="filter-count">{count}</span>}
        </Button>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content
            aria-label="Map filters"
            width="sm"
            maxW="calc(100vw - 24px)"
            maxH="calc(100dvh - 24px)"
            overflowY="auto"
          >
            <Popover.Body>
              <Stack gap="4">
                <Popover.Title>Map filters</Popover.Title>
                <Field.Root>
                  <Field.Label>Library folder</Field.Label>
                  <Select
                    value={query.rootId ?? ''}
                    onChange={(event) => onFilter({ rootId: event.target.value || null })}
                  >
                    <option value="">All folders</option>
                    {roots.map((root) => (
                      <option key={root.id} value={root.id}>
                        {root.label}
                        {root.availability === 'offline' ? ' · Offline' : ''}
                      </option>
                    ))}
                  </Select>
                </Field.Root>
                <Field.Root>
                  <Field.Label>From</Field.Label>
                  <Input
                    type="date"
                    value={query.fromDate}
                    onChange={(event) => onFilter({ fromDate: event.target.value })}
                  />
                </Field.Root>
                <Field.Root>
                  <Field.Label>Through</Field.Label>
                  <Input
                    type="date"
                    value={query.throughDate}
                    onChange={(event) => onFilter({ throughDate: event.target.value })}
                  />
                </Field.Root>
                <Button
                  type="button"
                  aria-pressed={unlocated}
                  onClick={() => {
                    onLocation();
                    setOpen(false);
                  }}
                >
                  No usable location
                  {unavailable === null ? '' : ` (${unavailable.toLocaleString()})`}
                </Button>
                <Popover.CloseTrigger asChild aria-label="Close filters">
                  <Button type="button">Close filters</Button>
                </Popover.CloseTrigger>
              </Stack>
            </Popover.Body>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
