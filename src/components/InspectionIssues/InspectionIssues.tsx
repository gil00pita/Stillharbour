import { Box, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type { InspectionIssuePage } from '../../bindings';
import { libraryData, errorMessage } from '../../ipc';
export function InspectionIssues({ jobId }: { jobId: string }) {
  const [page, setPage] = useState<InspectionIssuePage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  async function load(after: string | null = null) {
    const token = ++epoch.current;
    setBusy(true);
    try {
      const rows = await libraryData(
        { kind: 'inspection_issues', job_id: jobId, after },
        'inspection_issues',
      );
      if (token === epoch.current) {
        setPage(rows);
        setError(null);
      }
    } catch (failure) {
      if (token === epoch.current) setError(errorMessage(failure));
    } finally {
      if (token === epoch.current) setBusy(false);
    }
  }
  return (
    <chakra.section aria-label="Inspection issues">
      <Button disabled={busy} onClick={() => void load()}>
        Review retained inspection issues
      </Button>
      {error && <Text role="alert">{error}</Text>}
      {page && (
        <>
          <Box className="inspection-table-wrap">
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top">
                Incomplete, missing or unreadable scope · retained observations
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled>Entry</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Kind</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Inventory state</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Reason</Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {page.rows.map((issue, index) => (
                  <Table.Row unstyled key={index}>
                    <Table.ColumnHeader unstyled scope="row">
                      {issue.displayPath}
                    </Table.ColumnHeader>
                    <Table.Cell unstyled>{issue.entryKind}</Table.Cell>
                    <Table.Cell unstyled>{issue.inventory.replaceAll('_', ' ')}</Table.Cell>
                    <Table.Cell unstyled>{issue.error.message}</Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          {page.rows.length === 0 && <Text>No retained issues for this report.</Text>}
          {page.next && (
            <Button disabled={busy} onClick={() => void load(page.next)}>
              Next issue page
            </Button>
          )}
        </>
      )}
    </chakra.section>
  );
}
