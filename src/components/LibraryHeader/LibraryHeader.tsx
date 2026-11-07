import { Box, Heading, IconButton, Text, chakra } from '@chakra-ui/react';
import { PageHeader } from '@/components/PageHeader';
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
  onDisplay,
  displayOpen = false,
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
  onDisplay?: () => void;
  displayOpen?: boolean;
}) {
  return (
    <PageHeader>
      <chakra.header
        className="toolbar photo-toolbar"
        css={{
          flexShrink: '0',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '16px',
          minHeight: '64px',
          padding: '12px 24px',
          borderBottom: '1px solid {colors.border}',
          background: '{colors.bg.toolbar}',
          '@media (max-width: 600px)': { minHeight: '64px', padding: '18px 24px' },
          '& h1': { flex: '0 0 auto' },
          '& > .toolbar-secondary': {
            background: 'transparent',
            borderColor: 'transparent',
            color: '{colors.fg.muted}',
            padding: '7px 12px',
          },
          '& .jobs-trigger': {
            background: 'transparent',
            borderColor: 'transparent',
            color: '{colors.fg.muted}',
          },
          '& > .toolbar-secondary:hover:enabled': {
            background: '{colors.bg.hover}',
            color: 'var(--stillharbour-colors-fg, {colors.fg})',
          },
          '& .jobs-trigger:hover:enabled': {
            background: '{colors.bg.hover}',
            color: 'var(--stillharbour-colors-fg, {colors.fg})',
          },
          '@media (max-width: 760px)': {
            '& .toolbar-action-label': { display: 'none' },
            '& > .toolbar-secondary': { width: 'var(--control-height)', padding: '0' },
          },
        }}
      >
        {navigationControl}
        <Heading as="h1" unstyled>
          {section === 'Tags' ? 'Albums / Tags' : section}
        </Heading>
        <PlaceSearch
          value={value}
          onSearch={onSearch}
          selectedId={placeId}
          onChange={onPlace}
          disabled={busy}
          filterScope={mapScope}
        />
        <Box className="header-actions" css={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
          {onDisplay && (
            <IconButton
              aria-label="Display options"
              title="Display options"
              aria-haspopup="dialog"
              aria-expanded={displayOpen}
              aria-controls="library-display-panel"
              disabled={busy}
              onClick={onDisplay}
              width="control"
              height="control"
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                aria-hidden="true"
              >
                <rect x="3" y="3" width="7" height="7" rx="1.5" />
                <rect x="14" y="3" width="7" height="7" rx="1.5" />
                <rect x="3" y="14" width="7" height="7" rx="1.5" />
                <rect x="14" y="14" width="7" height="7" rx="1.5" />
              </svg>
            </IconButton>
          )}
          <IconButton
            onClick={onFilter}
            disabled={busy}
            aria-label={activeCount ? `Filter (${activeCount} active)` : 'Filter'}
            title={activeCount ? `Filter (${activeCount} active)` : 'Filter'}
            aria-haspopup="dialog"
            aria-expanded={filterOpen}
            aria-controls="library-filter-panel"
            position="relative"
            width="control"
            height="control"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M4 7h16M4 17h16M9 4v6m6 4v6"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
            {activeCount > 0 && (
              <Text
                as="span"
                className="filter-count"
                aria-hidden="true"
                position="absolute"
                top="-2px"
                right="-2px"
                minWidth="16px"
                height="16px"
                px="1"
                display="grid"
                placeItems="center"
                bg="brand.solid"
                color="brand.contrast"
                borderRadius="999px"
                fontSize="10px"
                lineHeight="1"
                pointerEvents="none"
              >
                {activeCount}
              </Text>
            )}
          </IconButton>
          {jobsControl}
          {onSelect && (
            <IconButton
              type="button"
              aria-label={selecting ? 'Done selecting' : 'Select'}
              title={selecting ? 'Done selecting' : 'Select'}
              aria-pressed={selecting}
              disabled={busy}
              onClick={onSelect}
              width="control"
              height="control"
            >
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
                focusable="false"
              >
                <rect x="3" y="3" width="18" height="18" rx="3" />
                <path d="m7 12 3 3 7-7" />
              </svg>
            </IconButton>
          )}
        </Box>
      </chakra.header>
    </PageHeader>
  );
}
