import { Button, Input } from './components/ui/controls';
// Loaded only by desktop-test entry code. This is visibly generated-fixture
// instrumentation, never a control for an existing photo library.
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type {
  GeneratedRenameRequest,
  GeneratedRenameReview,
  GeneratedWorkerStatus,
  OriginalOperationHistory,
} from './bindings';
import { errorMessage } from './ipc';
import { OriginalJobProgress } from './library/OriginalJobProgress';
import { OriginalHistoryReview } from './library/OriginalHistoryReview';

function readGeneratedHistory(jobId: string) {
  return invoke<OriginalOperationHistory>('test_generated_rename_history', { jobId });
}

export function GeneratedRenameLab() {
  const dialog = useRef<HTMLDialogElement>(null);
  const generation = useRef(0);
  const workerWasBusy = useRef(false);
  const [worker, setWorker] = useState<GeneratedWorkerStatus | null>(null);
  const [stopping, setStopping] = useState(false);
  const [open, setOpen] = useState(false);
  const [review, setReview] = useState<GeneratedRenameReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [pattern, setPattern] = useState('{event}_{sequence:03}_{original_stem}.{ext}');
  const [event, setEvent] = useState('Organized');
  const [acknowledged, setAcknowledged] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const show = () => setOpen(true);
    window.addEventListener('stillharbour:rename-lab', show);
    return () => {
      generation.current += 1;
      window.removeEventListener('stillharbour:rename-lab', show);
    };
  }, []);
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
  }, [open]);
  useEffect(() => {
    if (!open) return;
    let live = true;
    let first = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      const epoch = generation.current;
      try {
        const next = await invoke<GeneratedWorkerStatus>('test_generated_rename_worker', {
          request: { kind: 'status' },
        });
        if (!live || generation.current !== epoch) return;
        const finished = workerWasBusy.current && !next.busy;
        workerWasBusy.current = next.busy;
        setWorker(next);
        if (next.error && !next.stopRequested) setError(errorMessage(next.error));
        if (!next.busy && next.scopeCreated && (first || finished)) {
          setBusy(true);
          setAcknowledged(false);
          try {
            const inspected = await invoke<GeneratedRenameReview>('test_generated_rename', {
              request: { kind: 'inspect' },
            });
            if (live && generation.current === epoch) setReview(inspected);
          } finally {
            if (live && generation.current === epoch) setBusy(false);
          }
        }
      } catch (cause) {
        if (live && generation.current === epoch) setError(errorMessage(cause));
      } finally {
        first = false;
        // Exactly one request at a time; closing retires late responses.
        if (live) timer = setTimeout(() => void poll(), 500);
      }
    }
    void poll();
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [open]);
  async function request(value: GeneratedRenameRequest) {
    let id = ++generation.current;
    setBusy(true);
    setError(null);
    setAcknowledged(false);
    setReview((current) => (current ? { ...current, ticket: null } : null));
    try {
      const result = await invoke<GeneratedRenameReview>('test_generated_rename', {
        request: value,
      });
      if (generation.current === id) setReview(result);
      if (value.kind === 'reopen_retained' && generation.current === id) {
        const status = await invoke<GeneratedWorkerStatus>('test_generated_rename_worker', {
          request: { kind: 'status' },
        });
        if (generation.current !== id) return;
        // Retire every poll started during reconstruction before making the
        // fresh acknowledgment available. A late busy observation must not
        // manufacture another finished transition and clear that choice.
        id = ++generation.current;
        workerWasBusy.current = status.busy;
        setWorker(status);
      }
      if (value.kind === 'start' && generation.current === id) {
        const status = await invoke<GeneratedWorkerStatus>('test_generated_rename_worker', {
          request: { kind: 'status' },
        });
        if (generation.current !== id) return;
        workerWasBusy.current = status.busy;
        setWorker(status);
        if (!status.busy) {
          const inspected = await invoke<GeneratedRenameReview>('test_generated_rename', {
            request: { kind: 'inspect' },
          });
          if (generation.current === id) setReview(inspected);
        }
      }
    } catch (cause) {
      if (generation.current === id) setError(errorMessage(cause));
    } finally {
      if (generation.current === id) setBusy(false);
    }
  }
  async function stop() {
    if (!worker?.job) return;
    const epoch = generation.current;
    setStopping(true);
    setError(null);
    try {
      const status = await invoke<GeneratedWorkerStatus>('test_generated_rename_worker', {
        request: { kind: 'stop', job_id: worker.job.id },
      });
      if (generation.current !== epoch) return;
      workerWasBusy.current = status.busy;
      setWorker(status);
      if (!status.busy) {
        const inspected = await invoke<GeneratedRenameReview>('test_generated_rename', {
          request: { kind: 'inspect' },
        });
        if (generation.current === epoch) setReview(inspected);
      }
    } catch (cause) {
      if (generation.current === epoch) setError(errorMessage(cause));
    } finally {
      if (generation.current === epoch) setStopping(false);
    }
  }
  function close() {
    generation.current += 1;
    setBusy(false);
    setStopping(false);
    setWorker(null);
    workerWasBusy.current = false;
    setAcknowledged(false);
    setReview((current) => (current ? { ...current, ticket: null } : null));
    dialog.current?.close();
    setOpen(false);
  }
  if (!open) return null;
  return (
    <dialog
      ref={dialog}
      className="organization-preview"
      aria-labelledby="rename-lab-title"
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
    >
      <header className="organization-heading">
        <h2 id="rename-lab-title">Generated rename and undo fixture</h2>
        <Button onClick={close}>Close fixture review</Button>
      </header>
      <div className="organization-body">
        <p>
          Test instrumentation · disposable generated files only. Local retained versions are not
          independent recovery. Real-library mutations remain disabled.
        </p>
        {error && <p role="alert">{error} Inspect retained state before another action.</p>}
        {worker?.busy && (
          <section aria-label="Active generated worker">
            {worker.job && <OriginalJobProgress job={worker.job} />}
            <p aria-live="polite">
              {worker.job
                ? `${worker.completedMoves} / ${worker.totalMoves} journaled moves.`
                : 'Preparing queued worker.'}{' '}
              {worker.stopRequested
                ? 'Stop requested; finishing the current verification or journal boundary.'
                : 'Native worker active.'}
            </p>
            <Button
              disabled={stopping || worker.stopRequested || !worker.job}
              onClick={() => void stop()}
            >
              Stop after current step
            </Button>
          </section>
        )}
        <fieldset disabled={busy || stopping || worker?.busy}>
          {!review && !worker?.scopeCreated && (
            <>
              <Button onClick={() => void request({ kind: 'create' })}>
                Create generated rename fixture
              </Button>
              <label>
                Generated filename template
                <Input value={pattern} onChange={(e) => setPattern(e.target.value)} />
              </label>
              <label>
                Generated template event
                <Input value={event} onChange={(e) => setEvent(e.target.value)} />
              </label>
              <p>
                Five disposable payloads, including a same-stem XMP companion. No capture dates.
                Filename templates only; missing dates block.
              </p>
              <Button
                onClick={() =>
                  void request({
                    kind: 'create_from_template',
                    template: { pattern, event, missingDate: 'block' },
                  })
                }
              >
                Review generated template targets
              </Button>
            </>
          )}
          <Button onClick={() => void request({ kind: 'inspect' })}>
            Inspect retained rename state
          </Button>
          {worker?.scopeCreated && (
            <Button onClick={() => void request({ kind: 'reopen_retained' })}>
              Reopen owned generated scope
            </Button>
          )}
          {review && (
            <>
              <p role="status">
                {review.reverse ? 'Undo' : 'Rename'} · {review.state} · {review.completedSteps} /{' '}
                {review.totalSteps} journaled steps
              </p>
              <p>Plan: {review.planId}</p>
              <p>
                {review.catalogCommitted
                  ? 'Catalog paths committed with stable asset IDs.'
                  : 'Catalog paths await the verified native operation and atomic catalog commit.'}
              </p>
              {review.state === 'needs_catalog_commit' && (
                <p>
                  Native movement is complete. Fresh approval checks the files and commits catalog
                  paths without repeating moves.
                </p>
              )}
              {review.jobs.length > 0 && (
                <section aria-label="Durable organization jobs">
                  <h3>Durable organization jobs</h3>
                  {review.jobs
                    .filter((job) => !worker?.busy || job.id !== worker.job?.id)
                    .map((job) => (
                      <div key={job.id}>
                        <OriginalJobProgress job={job} />
                        <OriginalHistoryReview job={job} readHistory={readGeneratedHistory} />
                      </div>
                    ))}
                </section>
              )}
              <table aria-label="Generated rename targets">
                <thead>
                  <tr>
                    <th>Before</th>
                    <th>After</th>
                    <th>At last inspection</th>
                    <th>Catalog path</th>
                  </tr>
                </thead>
                <tbody>
                  {review.members.map((member) => (
                    <tr key={member.id}>
                      <td>{member.before}</td>
                      <td>{member.after}</td>
                      <td>{member.observed}</td>
                      <td>{member.catalogPath}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {review.state !== 'completed' && (
                <>
                  <label>
                    <input
                      type="checkbox"
                      checked={acknowledged}
                      disabled={Boolean(review.ticket)}
                      onChange={(event) => setAcknowledged(event.target.checked)}
                    />
                    I reviewed these exact generated-file targets and the recovery limits
                  </label>
                  {!review.ticket ? (
                    <Button
                      disabled={!acknowledged}
                      onClick={() =>
                        void request({ kind: 'approve', plan_id: review.planId, acknowledged })
                      }
                    >
                      Approve generated targets
                    </Button>
                  ) : (
                    <Button
                      onClick={() =>
                        void request({
                          kind: 'start',
                          plan_id: review.planId,
                          ticket: review.ticket!,
                        })
                      }
                    >
                      {review.state === 'needs_catalog_commit'
                        ? 'Commit reviewed generated catalog paths'
                        : `Run reviewed generated ${review.reverse ? 'undo' : 'rename'}`}
                    </Button>
                  )}
                </>
              )}
              {review.state === 'completed' && !review.reverse && (
                <Button onClick={() => void request({ kind: 'review_reverse' })}>
                  Review generated undo
                </Button>
              )}
            </>
          )}
        </fieldset>
        {busy && <p role="status">Checking generated native state…</p>}
      </div>
    </dialog>
  );
}
