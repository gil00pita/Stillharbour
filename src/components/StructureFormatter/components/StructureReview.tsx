import { Box, Flex, Heading, Text } from '@chakra-ui/react';
import { useEffect, useEffectEvent, useRef, useState } from 'react';
import { Button } from '@/components/Button';
import { SearchField } from '@/components/SearchField';
import type {
  LibraryOrganizationPreview,
  LibraryOrganizationSummary,
  LibraryStatus,
  OrganizationPreviewFilter,
  OrganizationPreviewQuery,
  OrganizationTemplate,
} from '../../../bindings';
import { StructureSummary } from './StructureSummary';
import { StructureReviewTable } from './StructureReviewTable';

const missingDateMessages = {
  skip: 'Items that need a date are skipped and kept in their current location.',
  block: 'Items that need a date are blocked by this saved rule.',
  undated:
    'Date-dependent paths use Undated and the original filename. Other issues still need review.',
};
function MissingDateNotice({
  summary,
  policy,
  onShow,
}: {
  summary: LibraryOrganizationSummary;
  policy: OrganizationTemplate['missingDate'];
  onShow: () => void;
}) {
  if (!summary.missingDates) return null;
  return (
    <Box className="structure-notice">
      <Box>
        <Text fontWeight="semibold">
          {summary.missingDates.toLocaleString()}{' '}
          {summary.missingDates === 1 ? 'item has' : 'items have'} no indexed capture date.
        </Text>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          {missingDateMessages[policy]}
        </Text>
      </Box>
      <Button type="button" onClick={onShow}>
        Show affected
      </Button>
    </Box>
  );
}
export function StructureReview({
  preview,
  template,
  roots,
  paging,
  stale,
  onPage,
  onEdit,
  error,
}: {
  preview: LibraryOrganizationPreview;
  template: OrganizationTemplate;
  roots: LibraryStatus['roots'];
  paging: boolean;
  stale: boolean;
  onPage: (offset: number, query: OrganizationPreviewQuery) => Promise<void>;
  onEdit: () => void;
  error: string | null;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  const caption = useRef<HTMLTableCaptionElement>(null);
  const focusPage = useRef(false);
  const initial = useRef(true);
  const [query, setQuery] = useState<OrganizationPreviewQuery>({ search: '', filter: 'all' });
  const [searching, setSearching] = useState(false);
  const load = useEffectEvent((next: OrganizationPreviewQuery) => onPage(0, next));
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
  }, []);
  useEffect(() => {
    if (initial.current) {
      initial.current = false;
      return;
    }
    let closed = false;
    const timeout = window.setTimeout(() => {
      void load(query).finally(() => {
        if (!closed) setSearching(false);
      });
    }, 180);
    return () => {
      closed = true;
      window.clearTimeout(timeout);
    };
  }, [query]);
  useEffect(() => {
    if (focusPage.current) {
      caption.current?.focus({ preventScroll: true });
      focusPage.current = false;
    }
  }, [preview, error]);
  function change(next: OrganizationPreviewQuery) {
    setSearching(true);
    setQuery(next);
  }
  const summary = preview.summary;
  const busy = paging || searching;
  const filters: { value: OrganizationPreviewFilter; label: string }[] = [
    { value: 'all', label: 'All photos' },
    { value: 'changes', label: 'Changes' },
    { value: 'needs_attention', label: 'Needs attention' },
    { value: 'skipped', label: 'Skipped' },
    ...(query.filter === 'missing_dates'
      ? [{ value: 'missing_dates' as const, label: 'Missing dates' }]
      : []),
  ];
  return (
    <Box as="section" aria-label="Proposed library structure">
      <Box className="structure-step-heading">
        <Button type="button" onClick={onEdit}>
          Edit structure
        </Button>
        <Heading as="h2" fontSize="2xl" mt="4" ref={heading} tabIndex={-1}>
          Review your changes
        </Heading>
        <Text color="fg.muted" mt="2">
          Before-and-after paths for your entire indexed library. This preview has not changed any
          files.
        </Text>
      </Box>
      {!summary ? (
        <Text role="alert">
          The native summary is unavailable. Return to editing and build a fresh preview.
        </Text>
      ) : (
        <>
          <StructureSummary summary={summary} />
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
            role="status"
          >
            {preview.total.toLocaleString()} library items · {preview.proposed.toLocaleString()}{' '}
            proposed · {preview.blocked.toLocaleString()} blocked ·{' '}
            {summary.skipped.toLocaleString()} skipped · {summary.unchanged.toLocaleString()}{' '}
            unchanged
          </Text>
          <MissingDateNotice
            summary={summary}
            policy={template.missingDate}
            onShow={() => change({ ...query, filter: 'missing_dates' })}
          />
          <Box className="structure-review-tools">
            <Box className="structure-filter-buttons" role="group" aria-label="Preview filters">
              {filters.map((filter) => (
                <Button
                  type="button"
                  key={filter.value}
                  aria-pressed={query.filter === filter.value}
                  disabled={stale}
                  onClick={() => change({ ...query, filter: filter.value })}
                >
                  {filter.label}
                </Button>
              ))}
            </Box>
            <Box className="structure-review-search">
              <SearchField
                label="Search preview"
                placeholder="Search photos or paths…"
                value={query.search}
                maxLength={256}
                disabled={stale}
                onChange={(search) => change({ ...query, search })}
              />
            </Box>
          </Box>
          <StructureReviewTable
            preview={preview}
            roots={roots}
            busy={busy}
            caption={caption}
            available={!error}
          />
          {!busy && preview.matching === 0 && (
            <Text my="5">
              {preview.total === 0
                ? 'No indexed photos or videos yet. Add a library location in Settings, then scan it.'
                : 'No items match this search and filter. Try All photos or clear the search.'}
            </Text>
          )}
          <Flex gap="3" mt="5" wrap="wrap" align="center" aria-label="Preview pages">
            <Button
              type="button"
              disabled={busy || stale || preview.offset === 0}
              onClick={() => {
                focusPage.current = true;
                void onPage(Math.max(0, preview.offset - 240), query);
              }}
            >
              Previous page
            </Button>
            <Text>
              Page {preview.matching ? Math.floor(preview.offset / 240) + 1 : 0} of{' '}
              {Math.ceil(preview.matching / 240)}
            </Text>
            <Button
              type="button"
              disabled={busy || stale || preview.offset + preview.rows.length >= preview.matching}
              onClick={() => {
                focusPage.current = true;
                void onPage(preview.offset + 240, query);
              }}
            >
              Next page
            </Button>
          </Flex>
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Search and filters cover the whole preview. Destination and companion checks remain
            pending; simulation covers all eligible items, including those outside the current
            filter.
          </Text>
        </>
      )}
    </Box>
  );
}
