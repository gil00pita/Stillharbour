import { useEffect, useRef, useState, type CSSProperties } from 'react';
import type { FolderPage, StorageSummary } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { displayBytes } from './format';
import { mapTiles, remainingBytes } from './treemap';

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
    <section className="inspection-storage" aria-labelledby="storage-totals-title">
      <h2 id="storage-totals-title">Storage in this inspection</h2>
      <dl className="inspection-metrics">
        <div>
          <dt>Logical file bytes</dt>
          <dd>{displayBytes(summary.total.logicalBytes)}</dd>
        </div>
        <div>
          <dt>Unique-object logical bytes</dt>
          <dd>{displayBytes(summary.total.uniqueObjectLogicalBytes)}</dd>
        </div>
        <div>
          <dt>Reported allocated bytes</dt>
          <dd>{displayBytes(summary.total.reportedAllocatedBytes)}</dd>
        </div>
        <div>
          <dt>Potential duplicate logical bytes</dt>
          <dd>{displayBytes(summary.potentialDuplicateLogicalBytes)}</dd>
        </div>
        <div>
          <dt>Physical reclaim estimate</dt>
          <dd>{displayBytes(summary.reclaimablePhysicalBytes)}</dd>
        </div>
      </dl>
      <p>
        Linked paths count separately in logical totals. Reported allocation does not establish
        physical reclaim on compressed, cloned or snapshotted storage. Moving to same-volume
        quarantine frees {displayBytes(summary.quarantineFreedBytes)}.
      </p>
      {summary.accountingIncomplete && (
        <p role="status">Object accounting reached its bound. Unknown metrics remain unmeasured.</p>
      )}
      <nav className="inspection-breadcrumbs" aria-label="Storage folders">
        {trail.map((node, index) => (
          <button
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
          </button>
        ))}
      </nav>
      {(error || geometryError) && (
        <div role="alert">
          <p>{error ?? geometryError}</p>
          {error && <button onClick={() => setRetry(retry + 1)}>Retry folder totals</button>}
        </div>
      )}
      {!current && !error && <p role="status">Loading retained folder totals…</p>}
      {current && (
        <>
          <p>
            Map area: observed logical file bytes. Small or zero-byte entries may have no visible
            area; the table retains their counts.
          </p>
          <div className="inspection-map" aria-label="Logical-byte storage map">
            {tiles.map((tile, index) => (
              <button
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
                <span>{tile.label}</span>
                <small>{displayBytes(tile.bytes)}</small>
              </button>
            ))}
          </div>
          <div className="inspection-table-wrap">
            <table>
              <caption>Folder totals for {current.parent.name}</caption>
              <thead>
                <tr>
                  <th>Folder</th>
                  <th>Files</th>
                  <th aria-sort={ascending ? 'ascending' : 'descending'}>
                    <button onClick={() => setAscending(!ascending)}>
                      Logical bytes · sort this page
                    </button>
                  </th>
                  <th>Unique-object logical bytes</th>
                  <th>Reported allocation</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <th scope="row">Total: {current.parent.name}</th>
                  <td>{current.parent.files.toLocaleString()}</td>
                  <td>{displayBytes(current.parent.logicalBytes)}</td>
                  <td>{displayBytes(current.parent.uniqueObjectLogicalBytes)}</td>
                  <td>{displayBytes(current.parent.reportedAllocatedBytes)}</td>
                </tr>
                <tr>
                  <th scope="row">Files directly in this folder</th>
                  <td>{current.parent.directFiles.toLocaleString()}</td>
                  <td>{displayBytes(current.parent.directLogicalBytes)}</td>
                  <td>Included in total</td>
                  <td>Included in total</td>
                </tr>
                {(ascending ? [...current.rows].reverse() : current.rows).map((row) => (
                  <tr key={row.id}>
                    <th scope="row">
                      <button onClick={() => open(row.id, row.name)}>{row.name}</button>
                    </th>
                    <td>{row.files.toLocaleString()}</td>
                    <td>{displayBytes(row.logicalBytes)}</td>
                    <td>{displayBytes(row.uniqueObjectLogicalBytes)}</td>
                    <td>{displayBytes(row.reportedAllocatedBytes)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="inspection-actions">
            <button
              disabled={!cursor}
              onClick={() => {
                epoch.current += 1;
                setCursor(null);
                setPage(null);
              }}
            >
              First folder page
            </button>
            <button
              disabled={!current.next}
              onClick={() => {
                epoch.current += 1;
                setCursor(current.next);
                setPage(null);
              }}
            >
              Next folder page
            </button>
          </div>
        </>
      )}
    </section>
  );
}
