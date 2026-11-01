// Loaded only by desktop-test entry code. This is visibly generated-fixture
// instrumentation, never a control for an existing photo library.
import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { GeneratedRenameRequest, GeneratedRenameReview } from './bindings';
import { errorMessage } from './ipc';

export function GeneratedRenameLab() {
  const dialog = useRef<HTMLDialogElement>(null);
  const generation = useRef(0);
  const [open, setOpen] = useState(false);
  const [review, setReview] = useState<GeneratedRenameReview | null>(null);
  const [busy, setBusy] = useState(false);
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
  async function request(value: GeneratedRenameRequest) {
    const id = ++generation.current;
    setBusy(true);
    setError(null);
    setAcknowledged(false);
    setReview((current) => (current ? { ...current, ticket: null } : null));
    try {
      const result = await invoke<GeneratedRenameReview>('test_generated_rename', {
        request: value,
      });
      if (generation.current === id) setReview(result);
    } catch (cause) {
      if (generation.current === id) setError(errorMessage(cause));
    } finally {
      if (generation.current === id) setBusy(false);
    }
  }
  function close() {
    generation.current += 1;
    setBusy(false);
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
        <button onClick={close}>Close fixture review</button>
      </header>
      <div className="organization-body">
        <p>
          Test instrumentation · disposable generated files only. Local retained versions are not
          independent recovery. Real-library mutations remain disabled.
        </p>
        {error && <p role="alert">{error} Inspect retained state before another action.</p>}
        <fieldset disabled={busy}>
          {!review && (
            <button onClick={() => void request({ kind: 'create' })}>
              Create generated rename fixture
            </button>
          )}
          <button onClick={() => void request({ kind: 'inspect' })}>
            Inspect retained rename state
          </button>
          {review && (
            <>
              <p role="status">
                {review.reverse ? 'Undo' : 'Rename'} · {review.state} · {review.completedSteps} /{' '}
                {review.totalSteps} journaled steps
              </p>
              <p>Plan: {review.planId}</p>
              <table aria-label="Generated rename targets">
                <thead>
                  <tr>
                    <th>Before</th>
                    <th>After</th>
                    <th>Observed now</th>
                  </tr>
                </thead>
                <tbody>
                  {review.members.map((member) => (
                    <tr key={member.id}>
                      <td>{member.before}</td>
                      <td>{member.after}</td>
                      <td>{member.observed}</td>
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
                    <button
                      disabled={!acknowledged}
                      onClick={() =>
                        void request({ kind: 'approve', plan_id: review.planId, acknowledged })
                      }
                    >
                      Approve generated targets
                    </button>
                  ) : (
                    <button
                      onClick={() =>
                        void request({
                          kind: 'start',
                          plan_id: review.planId,
                          ticket: review.ticket!,
                        })
                      }
                    >
                      Run reviewed generated {review.reverse ? 'undo' : 'rename'}
                    </button>
                  )}
                </>
              )}
              {review.state === 'completed' && !review.reverse && (
                <button onClick={() => void request({ kind: 'review_reverse' })}>
                  Review generated undo
                </button>
              )}
            </>
          )}
        </fieldset>
        {busy && <p role="status">Checking generated native state…</p>}
      </div>
    </dialog>
  );
}
