import { Box, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { useState } from 'react';
import type { InspectionFinding } from '../../bindings';
import { Button } from '@/components/Button';

function needsAttention(row: InspectionFinding) {
  return (
    row.error !== null ||
    row.inventory !== 'present' ||
    [
      'suspected_corrupt',
      'confirmed_decode_failure',
      'changed_during_read',
      'decoder_warning',
    ].includes(row.content) ||
    row.checksum === 'changed_since_baseline'
  );
}
export function IntegrityFindings({ rows }: { rows: InspectionFinding[] }) {
  const [attention, setAttention] = useState(false);
  const flagged = rows.filter(needsAttention);
  const visible = attention ? flagged : rows;
  return (
    <chakra.section className="integrity-findings" aria-label="File health findings">
      <Heading as="h2" unstyled>
        File health findings
      </Heading>
      <Text>
        Review changed checksums, unreadable files and incomplete checks separately. A checksum
        mismatch means changed since baseline; it does not prove damage.
      </Text>
      <chakra.dl
        className="inspection-metrics"
        css={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          gap: '12px',
          '& > div': { border: '1px solid {colors.border}', padding: '16px', borderRadius: '8px' },
          '& dt': { color: '{colors.fg.muted}', fontSize: '12px' },
          '& dd': { margin: '8px 0 0', fontSize: '20px', overflowWrap: 'anywhere' },
          '& > div:first-child': { gridColumn: '1 / -1', border: '0', paddingInline: '0' },
          '& > div:first-child dd': { fontSize: '28px' },
        }}
      >
        <Box>
          <dt>Files on this page</dt>
          <dd>{rows.length}</dd>
        </Box>
        <Box>
          <dt>Need attention</dt>
          <dd>{flagged.length}</dd>
        </Box>
        <Box>
          <dt>Not checked</dt>
          <dd>{rows.filter((row) => row.content === 'not_checked').length}</dd>
        </Box>
        <Box>
          <dt>Unavailable</dt>
          <dd>
            {
              rows.filter((row) => row.inventory === 'offline' || row.inventory === 'inaccessible')
                .length
            }
          </dd>
        </Box>
      </chakra.dl>
      <Box
        className="inspection-actions"
        css={{ display: 'flex', flexWrap: 'wrap', gap: '8px 20px', alignItems: 'center' }}
        role="group"
        aria-label="Health finding filters"
      >
        <Button aria-pressed={!attention} onClick={() => setAttention(false)}>
          All findings
        </Button>
        <Button aria-pressed={attention} onClick={() => setAttention(true)}>
          Needs attention
        </Button>
      </Box>
      <Box
        className="inspection-table-wrap"
        css={{
          width: '100%',
          overflow: 'auto',
          margin: '16px 0',
          '& table': { width: '100%', borderCollapse: 'collapse', fontSize: '13px' },
          '& caption': { textAlign: 'left', marginBottom: '8px' },
          '& th': {
            textAlign: 'left',
            verticalAlign: 'top',
            padding: '12px',
            borderBottom: '1px solid {colors.border}',
            fontWeight: '500',
          },
          '& td': {
            textAlign: 'left',
            verticalAlign: 'top',
            padding: '12px',
            borderBottom: '1px solid {colors.border}',
          },
          '& th small': {
            display: 'block',
            maxWidth: '32rem',
            overflowWrap: 'anywhere',
            fontWeight: '400',
            color: '{colors.fg.muted}',
          },
        }}
      >
        <Table.Root unstyled>
          <Table.Caption unstyled captionSide="top">
            Retained health observations · current member page
          </Table.Caption>
          <Table.Header unstyled>
            <Table.Row unstyled>
              <Table.ColumnHeader unstyled>File</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Presence</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Content check</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Checksum</Table.ColumnHeader>
              <Table.ColumnHeader unstyled>Coverage and next step</Table.ColumnHeader>
            </Table.Row>
          </Table.Header>
          <Table.Body unstyled>
            {visible.map((row) => (
              <Table.Row unstyled key={row.id}>
                <Table.ColumnHeader unstyled scope="row">
                  <bdi>{row.name}</bdi>
                  <small>
                    <bdi>{row.displayPath}</bdi>
                  </small>
                </Table.ColumnHeader>
                <Table.Cell unstyled>{row.inventory.replaceAll('_', ' ')}</Table.Cell>
                <Table.Cell unstyled>{row.content.replaceAll('_', ' ')}</Table.Cell>
                <Table.Cell unstyled>{row.checksum.replaceAll('_', ' ')}</Table.Cell>
                <Table.Cell unstyled>
                  {row.error?.message ?? row.coverage}
                  {row.checksum === 'changed_since_baseline' && (
                    <Text>
                      Preserve this version and compare with independent recovery. Baseline history
                      is retained.
                    </Text>
                  )}
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table.Root>
        {!visible.length && (
          <Text>
            No findings match this filter on the loaded page. Uninspected scope remains unknown.
          </Text>
        )}
      </Box>
    </chakra.section>
  );
}
