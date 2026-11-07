import { Box, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState, type CSSProperties } from 'react';
import type { FolderPage, StorageSummary } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { displayBytes } from '../../inspection/format';
import { mapTiles, remainingBytes } from '../../inspection/treemap';

export function StorageReview({ jobId, summary }: { jobId: string; summary: StorageSummary }) {
  const [trail, setTrail] = useState([{ id: 'scope', name: 'Selected scope' }]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [page, setPage] = useState<FolderPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [ascending, setAscending] = useState(false);
  const epoch = useRef(0);
  const parent = trail.at(-1)?.id ?? 'scope';
  useEffect(() => {
    const token = ++epoch.current;
    void libraryData(
      { kind: 'inspection_folders', job_id: jobId, parent_id: parent, after: cursor },
      'inspection_folders',
    )
      .then((result) => {
        if (token !== epoch.current) return;
        setPage(result);
        setError(null);
      })
      .catch((failure) => {
        if (token === epoch.current) setError(errorMessage(failure));
      });
    return () => {
      epoch.current += 1;
    };
  }, [jobId, parent, cursor, retry]);
  const current = page?.parent.id === parent ? page : null;
  let geometryError: string | null = null;
  let tiles: ReturnType<typeof mapTiles> = [];
  if (current)
    try {
      const values = current.rows.map((row) => ({
        id: row.id,
        label: row.name,
        bytes: row.logicalBytes,
      }));
      values.push({
        id: 'direct-files',
        label: 'Files in this folder',
        bytes: current.parent.directLogicalBytes,
      });
      values.push({
        id: 'other-folders',
        label: 'Other folders on other pages',
        bytes: remainingBytes(
          current.parent.logicalBytes,
          values.map((value) => value.bytes),
        ),
      });
      tiles = mapTiles(values);
    } catch (failure) {
      geometryError = errorMessage(failure);
    }
  function open(id: string, name: string) {
    epoch.current += 1;
    setCursor(null);
    setPage(null);
    setError(null);
    setTrail([...trail, { id, name }]);
  }
  return (
    <chakra.section className="inspection-storage" aria-labelledby="storage-totals-title">
      <Heading as="h2" unstyled id="storage-totals-title">
        Storage in this inspection
      </Heading>
      <Text data-bytes={summary.total.logicalBytes}>
        Total file size: {displayBytes(summary.total.logicalBytes)}. Logical sizes count hard-linked
        paths separately; allocation is not recoverable space.
      </Text>
      <chakra.nav className="inspection-breadcrumbs" aria-label="Storage folders">
        {trail.map((node, index) => (
          <Button
            key={node.id}
            aria-current={index === trail.length - 1 ? 'location' : undefined}
            onClick={() => {
              epoch.current += 1;
              setCursor(null);
              setPage(null);
              setTrail(trail.slice(0, index + 1));
            }}
          >
            {node.name}
          </Button>
        ))}
      </chakra.nav>
      {(error || geometryError) && (
        <Box role="alert">
          <Text>{error ?? geometryError}</Text>
          {error && <Button onClick={() => setRetry(retry + 1)}>Retry folder totals</Button>}
        </Box>
      )}
      {!current && !error && <Text role="status">Loading retained folder totals…</Text>}
      {current && (
        <>
          <Text>
            Map area: observed logical file bytes. Small or zero-byte entries may have no visible
            area; the table retains their counts.
          </Text>
          <Box className="inspection-map" aria-label="Logical-byte storage map">
            {tiles.map((tile, index) => (
              <Button
                key={tile.id}
                style={
                  {
                    left: `${tile.x}%`,
                    top: `${tile.y * 2}%`,
                    width: `${tile.width}%`,
                    height: `${tile.height * 2}%`,
                    '--map-hue': String(205 + (index % 7) * 8),
                  } as CSSProperties
                }
                disabled={tile.id === 'direct-files' || tile.id === 'other-folders'}
                aria-label={`${tile.label}: ${displayBytes(tile.bytes)}`}
                title={`${tile.label} · ${displayBytes(tile.bytes)}`}
                onClick={() => open(tile.id, tile.label)}
              >
                <Text as="span">{tile.label}</Text>
                <small>{displayBytes(tile.bytes)}</small>
              </Button>
            ))}
          </Box>
          <Box className="inspection-table-wrap">
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top">
                Folder totals for {current.parent.name}
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled>Folder</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Files</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled aria-sort={ascending ? 'ascending' : 'descending'}>
                    <Button onClick={() => setAscending(!ascending)}>
                      File size · sort this page
                    </Button>
                  </Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Hard links counted once</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Reported allocation</Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled scope="row">
                    Total: {current.parent.name}
                  </Table.ColumnHeader>
                  <Table.Cell unstyled>{current.parent.files.toLocaleString()}</Table.Cell>
                  <Table.Cell unstyled data-bytes={current.parent.logicalBytes}>
                    {displayBytes(current.parent.logicalBytes)}
                  </Table.Cell>
                  <Table.Cell unstyled>
                    {displayBytes(current.parent.uniqueObjectLogicalBytes)}
                  </Table.Cell>
                  <Table.Cell unstyled>
                    {displayBytes(current.parent.reportedAllocatedBytes)}
                  </Table.Cell>
                </Table.Row>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled scope="row">
                    Files directly in this folder
                  </Table.ColumnHeader>
                  <Table.Cell unstyled>{current.parent.directFiles.toLocaleString()}</Table.Cell>
                  <Table.Cell unstyled>
                    {displayBytes(current.parent.directLogicalBytes)}
                  </Table.Cell>
                  <Table.Cell unstyled>Included in total</Table.Cell>
                  <Table.Cell unstyled>Included in total</Table.Cell>
                </Table.Row>
                {(ascending ? [...current.rows].reverse() : current.rows).map((row) => (
                  <Table.Row unstyled key={row.id}>
                    <Table.ColumnHeader unstyled scope="row">
                      <Button onClick={() => open(row.id, row.name)}>{row.name}</Button>
                    </Table.ColumnHeader>
                    <Table.Cell unstyled>{row.files.toLocaleString()}</Table.Cell>
                    <Table.Cell unstyled>{displayBytes(row.logicalBytes)}</Table.Cell>
                    <Table.Cell unstyled>{displayBytes(row.uniqueObjectLogicalBytes)}</Table.Cell>
                    <Table.Cell unstyled>{displayBytes(row.reportedAllocatedBytes)}</Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Box className="inspection-actions">
            <Button
              disabled={!cursor}
              onClick={() => {
                epoch.current += 1;
                setCursor(null);
                setPage(null);
              }}
            >
              First folder page
            </Button>
            <Button
              disabled={!current.next}
              onClick={() => {
                epoch.current += 1;
                setCursor(current.next);
                setPage(null);
              }}
            >
              Next folder page
            </Button>
          </Box>
        </>
      )}
    </chakra.section>
  );
}
