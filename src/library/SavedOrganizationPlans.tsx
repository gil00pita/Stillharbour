import { useEffect, useRef, useState } from 'react';
import type {
  OrganizationPlanDetail,
  OrganizationPlanPage,
  PlanExport,
  PlanExportFormat,
  PlanExportSaved,
  OrganizationRow,
  BundleConfirmation,
} from '../bindings';
import { errorMessage, libraryData, pickDirectory } from '../ipc';
import { OrganizationTrees } from './OrganizationTrees';
import { BundleInventoryReview } from './BundleInventoryReview';
import { CopyManifestReview } from './CopyManifestReview';

export function SavedOrganizationPlans({ onClose }: { onClose: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const request = useRef(0);
  const [page, setPage] = useState<OrganizationPlanPage | null>(null);
  const [detail, setDetail] = useState<OrganizationPlanDetail | null>(null);
  const [exported, setExported] = useState<PlanExport | null>(null);
  const [savedExport, setSavedExport] = useState<PlanExportSaved | null>(null);
  const [companionRow, setCompanionRow] = useState<OrganizationRow | null>(null);
  const [companionChoices, setCompanionChoices] = useState<Record<string, BundleConfirmation>>({});
  const companionOpener = useRef<HTMLButtonElement | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  async function list(before: string | null = null) {
    const id = ++request.current;
    setBusy(true);
    setError(null);
    try {
      const result = await libraryData(
        { kind: 'organization_plans', before },
        'organization_plans',
      );
      if (id === request.current)
        setPage((old) => ({
          ...result,
          plans: before ? [...(old?.plans ?? []), ...result.plans] : result.plans,
        }));
    } catch (error) {
      if (id === request.current) setError(errorMessage(error));
    } finally {
      if (id === request.current) setBusy(false);
    }
  }
  useEffect(() => {
    const element = dialog.current;
    const opener = document.activeElement;
    element?.showModal();
    const id = ++request.current;
    void libraryData({ kind: 'organization_plans', before: null }, 'organization_plans')
      .then((result) => {
        if (id === request.current) setPage(result);
      })
      .catch((error: unknown) => {
        if (id === request.current) setError(errorMessage(error));
      })
      .finally(() => {
        if (id === request.current) setBusy(false);
      });
    return () => {
      request.current += 1;
      element?.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  async function open(id: string) {
    const token = ++request.current;
    setBusy(true);
    setError(null);
    setDetail(null);
    setExported(null);
    setSavedExport(null);
    setCompanionRow(null);
    setCompanionChoices({});
    try {
      const result = await libraryData(
        { kind: 'organization_plan', plan_id: id },
        'organization_plan',
      );
      if (token === request.current) setDetail(result);
    } catch (error) {
      if (token === request.current) setError(errorMessage(error));
    } finally {
      if (token === request.current) setBusy(false);
    }
  }
  async function exportText(format: PlanExportFormat) {
    if (!detail) return;
    const token = ++request.current;
    setBusy(true);
    setError(null);
    setExported(null);
    try {
      const result = await libraryData(
        { kind: 'organization_plan_export', plan_id: detail.plan.id, format },
        'organization_plan_export',
      );
      if (token === request.current) setExported(result);
    } catch (error) {
      if (token === request.current) setError(errorMessage(error));
    } finally {
      if (token === request.current) setBusy(false);
    }
  }
  async function saveFile(format: PlanExportFormat) {
    if (!detail || busy) return;
    const token = ++request.current;
    setBusy(true);
    setError(null);
    try {
      const selected = await pickDirectory();
      if (token !== request.current || !selected) return;
      if (!selected.identityAvailable)
        throw new Error('Choose an export folder with a verifiable native identity.');
      setSavedExport(null);
      const result = await libraryData(
        {
          kind: 'organization_plan_save_export',
          plan_id: detail.plan.id,
          format,
          selection_id: selected.id,
        },
        'organization_plan_export_saved',
      );
      if (token === request.current) setSavedExport(result);
    } catch (error) {
      if (token === request.current) setError(errorMessage(error));
    } finally {
      if (token === request.current) setBusy(false);
    }
  }
  return (
    <dialog
      className="organization-preview"
      ref={dialog}
      aria-labelledby="saved-plans-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <header className="organization-heading">
        <h2 id="saved-plans-title">Saved organization plans</h2>
        <button onClick={onClose}>Close saved plans</button>
      </header>
      <div className="organization-body" aria-busy={busy}>
        <p>
          Retained drafts preserve their original paths and template versions. Copying remains
          unavailable.
        </p>
        <button disabled={busy} onClick={() => void list()}>
          Refresh plans
        </button>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {page?.plans.length === 0 && (
          <p>
            No saved plans yet. Select photos, build an organization preview and choose a
            destination.
          </p>
        )}
        <ul>
          {page?.plans.map((plan) => (
            <li key={plan.id}>
              <button disabled={busy} onClick={() => void open(plan.id)}>
                <bdi>{plan.destinationPath}</bdi> · {plan.proposed} proposed · {plan.createdAt}
              </button>
            </li>
          ))}
        </ul>
        {page?.next && (
          <button disabled={busy} onClick={() => void list(page.next)}>
            Load more plans
          </button>
        )}
        {detail && (
          <section aria-label="Saved plan detail">
            <h3>Draft · {detail.plan.createdAt}</h3>
            <p>
              <bdi>{detail.plan.destinationPath}</bdi>
            </p>
            {detail.stale && (
              <p role="alert">
                The catalog changed. This retained draft needs a fresh preview before any future
                approval.
              </p>
            )}
            <p>
              Destination identity was checked when saved. It is not reserved or revalidated by
              reopening this draft.
            </p>
            <p>
              {detail.plan.preview.proposed} proposed · {detail.plan.preview.blocked} blocked ·{' '}
              {BigInt(detail.plan.preview.selectedBytes).toLocaleString()} selected file bytes
            </p>
            {detail.plan.templateReference && (
              <p>Template version {detail.plan.templateReference.version}</p>
            )}
            <div className="organization-table">
              <OrganizationTrees rows={detail.plan.preview.rows} />
              <table>
                <caption>Retained source and destination paths</caption>
                <thead>
                  <tr>
                    <th>Source</th>
                    <th>Destination relative path</th>
                  </tr>
                </thead>
                <tbody>
                  {detail.plan.preview.rows.map((row) => (
                    <tr key={row.assetId}>
                      <td>
                        <bdi>{row.sourcePath}</bdi>
                        <button
                          type="button"
                          disabled={busy || detail.stale || row.issues.length > 0}
                          onClick={(event) => {
                            companionOpener.current = event.currentTarget;
                            setCompanionRow(row);
                          }}
                        >
                          Review all companion filenames
                        </button>
                        {companionChoices[row.assetId] && (
                          <p className="help">Chosen review: {companionChoices[row.assetId]?.id}</p>
                        )}
                      </td>
                      <td>
                        <bdi>{row.targetRelativePath ?? 'Blocked'}</bdi>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            {companionRow && (
              <BundleInventoryReview
                key={`${detail.plan.id}:${companionRow.assetId}`}
                planId={detail.plan.id}
                row={companionRow}
                catalogRevision={detail.plan.preview.catalogRevision}
                stale={detail.stale}
                onChoose={(decision) => {
                  if (
                    decision.planId === detail.plan.id &&
                    decision.assetId === companionRow.assetId
                  )
                    setCompanionChoices((previous) => ({
                      ...previous,
                      [decision.assetId]: decision,
                    }));
                }}
                onClose={() => {
                  setCompanionRow(null);
                  if (companionOpener.current?.isConnected) companionOpener.current.focus();
                }}
              />
            )}
            <CopyManifestReview key={detail.plan.id} detail={detail} choices={companionChoices} />
            <p className="help">
              Native target names, complete companion inventory, capacity, metadata fidelity and
              execution recovery checks remain pending.
            </p>
            <p className="help">Fingerprint: {detail.plan.digest}</p>
            <button disabled={busy} onClick={() => void exportText('json')}>
              View JSON export
            </button>
            <button disabled={busy} onClick={() => void exportText('csv')}>
              View CSV export
            </button>
            <p className="help">
              Save a new review file to a native-selected folder outside your libraries. The
              generated filename is preserved; an existing file is never replaced. Exports contain
              review data, not original photos or execution approval.
            </p>
            <button disabled={busy} onClick={() => void saveFile('json')}>
              Save JSON file…
            </button>
            <button disabled={busy} onClick={() => void saveFile('csv')}>
              Save CSV file…
            </button>
            {savedExport && (
              <p role="status">
                Review file saved: <bdi>{savedExport.path}</bdi> ·{' '}
                {BigInt(savedExport.bytes).toLocaleString()} bytes verified.
              </p>
            )}
            {exported && (
              <label>
                Export text — {exported.filename}
                <textarea
                  rows={10}
                  readOnly
                  value={exported.text}
                  onFocus={(event) => event.currentTarget.select()}
                />
              </label>
            )}
          </section>
        )}
      </div>
    </dialog>
  );
}
