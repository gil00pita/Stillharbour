import { Box, Heading, IconButton, Text, chakra } from '@chakra-ui/react';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import type { ReactNode } from 'react';
import type { Section } from '../../library/model';
import { PlaceSearch } from '@/components/PlaceSearch';
import type { PlaceSummary } from '../../bindings';
import { ThumbnailScale } from '@/components/ThumbnailScale';
import type { GalleryDisplay } from '@/components/FilterPanel';
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
  thumbnailSize,
  onThumbnailSize,
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
  thumbnailSize?: GalleryDisplay['size'] | undefined;
  onThumbnailSize?: (size: GalleryDisplay['size']) => void;
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
        {thumbnailSize && onThumbnailSize && (
          <ThumbnailScale size={thumbnailSize} onChange={onThumbnailSize} />
        )}
        <Box className="header-actions" css={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
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
            Filter
            {activeCount > 0 && (
              <Text
                as="span"
                className="filter-count"
                css={{ marginLeft: '8px', color: '{colors.brand.solid}' }}
              >
                {activeCount}
              </Text>
            )}
          </Button>
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
