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
    <section className="integrity-findings" aria-label="File health findings">
      <h2>File health findings</h2>
      <p>
        Review changed checksums, unreadable files and incomplete checks separately. A checksum
        mismatch means changed since baseline; it does not prove damage.
      </p>
      <dl className="inspection-metrics">
        <div>
          <dt>Files on this page</dt>
          <dd>{rows.length}</dd>
        </div>
        <div>
          <dt>Need attention</dt>
          <dd>{flagged.length}</dd>
        </div>
        <div>
          <dt>Not checked</dt>
          <dd>{rows.filter((row) => row.content === 'not_checked').length}</dd>
        </div>
        <div>
          <dt>Unavailable</dt>
          <dd>
            {
              rows.filter((row) => row.inventory === 'offline' || row.inventory === 'inaccessible')
                .length
            }
          </dd>
        </div>
      </dl>
      <div className="inspection-actions" role="group" aria-label="Health finding filters">
        <Button aria-pressed={!attention} onClick={() => setAttention(false)}>
          All findings
        </Button>
        <Button aria-pressed={attention} onClick={() => setAttention(true)}>
          Needs attention
        </Button>
      </div>
      <div className="inspection-table-wrap">
        <table>
          <caption>Retained health observations · current member page</caption>
          <thead>
            <tr>
              <th>File</th>
              <th>Presence</th>
              <th>Content check</th>
              <th>Checksum</th>
              <th>Coverage and next step</th>
            </tr>
          </thead>
          <tbody>
            {visible.map((row) => (
              <tr key={row.id}>
                <th scope="row">
                  <bdi>{row.name}</bdi>
                  <small>
                    <bdi>{row.displayPath}</bdi>
                  </small>
                </th>
                <td>{row.inventory.replaceAll('_', ' ')}</td>
                <td>{row.content.replaceAll('_', ' ')}</td>
                <td>{row.checksum.replaceAll('_', ' ')}</td>
                <td>
                  {row.error?.message ?? row.coverage}
                  {row.checksum === 'changed_since_baseline' && (
                    <p>
                      Preserve this version and compare with independent recovery. Baseline history
                      is retained.
                    </p>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        {!visible.length && (
          <p>
            No findings match this filter on the loaded page. Uninspected scope remains unknown.
          </p>
        )}
      </div>
    </section>
  );
}
