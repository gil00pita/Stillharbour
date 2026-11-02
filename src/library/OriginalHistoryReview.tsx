import { Button } from '../components/ui/controls';
import { useEffect, useRef, useState } from 'react';
import type { OriginalOperationHistory, OriginalOperationJob } from '../bindings';
import { errorMessage, libraryData } from '../ipc';

async function readCatalogHistory(jobId: string) {
  return libraryData(
    { kind: 'organization_original_history', job_id: jobId },
    'organization_original_history',
  );
}
type Props = {
  job: OriginalOperationJob;
  readHistory?: (jobId: string) => Promise<OriginalOperationHistory>;
};
export function OriginalHistoryReview(props: Props) {
  return <HistoryForJob key={props.job.id} {...props} />;
}
function HistoryForJob({ job, readHistory = readCatalogHistory }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const epoch = useRef(0);
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [history, setHistory] = useState<OriginalOperationHistory | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
  }, [open]);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  function close() {
    epoch.current += 1;
    dialog.current?.close();
    setOpen(false);
    setPending(false);
    setHistory(null);
    setError(null);
  }
  async function load() {
    const token = ++epoch.current;
    setOpen(true);
    setPending(true);
    setHistory(null);
    setError(null);
    setPage(0);
    try {
      const value = await readHistory(job.id);
      if (epoch.current !== token) return;
      if (
        value.job.id !== job.id ||
        value.job.planId !== job.planId ||
        value.job.sourceRootId !== job.sourceRootId ||
        !value.historyOnly ||
        value.currentLocationsChecked ||
        value.executionAvailable ||
        value.members.length === 0 ||
        value.members.length > 512 ||
        new Set(value.members.map((member) => member.id)).size !== value.members.length
      ) {
        throw new Error(
          'Organization history does not match this job. Preserve the catalog for recovery review.',
        );
      }
      setHistory(value);
    } catch (cause) {
      if (epoch.current === token) setError(errorMessage(cause));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  return (
    <>
      <Button type="button" onClick={() => void load()}>
        Review retained organization plan
      </Button>
      {open && (
        <dialog
          ref={dialog}
          className="organization-preview"
          aria-labelledby="original-history-title"
          onCancel={(event) => {
            event.preventDefault();
            event.stopPropagation();
            close();
          }}
        >
          <header className="organization-heading">
            <h2 id="original-history-title">Retained organization plan</h2>
            <Button type="button" onClick={close}>
              Close plan history
            </Button>
          </header>
          <div className="organization-body">
            <p>
              Saved plan history. Current file locations and contents require a fresh native
              recovery inspection before any action.
            </p>
            <p className="help">
              Opening this review grants no execution approval. Original-file changes remain
              unavailable under the preservation recovery gate.
            </p>
            {pending && <p role="status">Loading retained plan…</p>}
            {error && (
              <>
                <p role="alert" className="error">
                  {error}
                </p>
                <Button type="button" onClick={() => void load()}>
                  Retry history
                </Button>
              </>
            )}
            {history && (
              <>
                <p>
                  {history.job.state === 'needs_recovery'
                    ? 'Needs recovery review'
                    : history.job.state === 'completed_verified'
                      ? 'Recorded completed and verified'
                      : 'Recorded pending attempt'}
                </p>
                <p>
                  {history.members.length} retained members · {history.job.verifiedMoves} /{' '}
                  {history.job.totalMoves} recorded verified moves
                </p>
                <p>
                  Plan digest: <bdi>{history.planDigest}</bdi>
                </p>
                {history.reverseOf && (
                  <p>
                    Reverse of plan <bdi>{history.reverseOf}</bdi>
                  </p>
                )}
                <div className="organization-table">
                  <table>
                    <caption>
                      Saved paths for this {history.job.kind === 'undo' ? 'undo' : 'rename'} plan
                    </caption>
                    <thead>
                      <tr>
                        <th scope="col">Before</th>
                        <th scope="col">After</th>
                      </tr>
                    </thead>
                    <tbody>
                      {history.members.slice(page * 64, (page + 1) * 64).map((member) => (
                        <tr key={member.id}>
                          <td>
                            <bdi>{member.before}</bdi>
                          </td>
                          <td>
                            <bdi>{member.after}</bdi>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {history.members.length > 64 && (
                  <nav aria-label="Retained plan members">
                    <Button
                      type="button"
                      disabled={page === 0}
                      onClick={() => setPage((value) => value - 1)}
                    >
                      Previous members
                    </Button>
                    <span>
                      Page {page + 1} of {Math.ceil(history.members.length / 64)}
                    </span>
                    <Button
                      type="button"
                      disabled={(page + 1) * 64 >= history.members.length}
                      onClick={() => setPage((value) => value + 1)}
                    >
                      Next members
                    </Button>
                  </nav>
                )}
              </>
            )}
          </div>
        </dialog>
      )}
    </>
  );
}
