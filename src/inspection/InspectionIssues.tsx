import { useEffect, useRef, useState } from 'react';
import type { InspectionIssuePage } from '../bindings';
import { libraryData, errorMessage } from '../ipc';
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
    <section aria-label="Inspection issues">
      <button disabled={busy} onClick={() => void load()}>
        Review retained inspection issues
      </button>
      {error && <p role="alert">{error}</p>}
      {page && (
        <>
          <div className="inspection-table-wrap">
            <table>
              <caption>Incomplete, missing or unreadable scope · retained observations</caption>
              <thead>
                <tr>
                  <th>Entry</th>
                  <th>Kind</th>
                  <th>Inventory state</th>
                  <th>Reason</th>
                </tr>
              </thead>
              <tbody>
                {page.rows.map((issue, index) => (
                  <tr key={index}>
                    <th scope="row">{issue.displayPath}</th>
                    <td>{issue.entryKind}</td>
                    <td>{issue.inventory.replaceAll('_', ' ')}</td>
                    <td>{issue.error.message}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {page.rows.length === 0 && <p>No retained issues for this report.</p>}
          {page.next && (
            <button disabled={busy} onClick={() => void load(page.next)}>
              Next issue page
            </button>
          )}
        </>
      )}
    </section>
  );
}
