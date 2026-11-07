import { Field, Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type {
  CopyManifest,
  ImportBundleChoice,
  ImportComparisonJob,
  ImportHistory,
  ImportPlan,
  ImportSession,
  ImportSkipApproval,
  ImportCopyReceipt,
} from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { CopyIoControls } from '@/components/CopyJobProgress';
import { VerifiedCopyWorkflow } from '@/components/VerifiedCopyWorkflow';
import { ImportOutputIndexReview } from '@/components/ImportOutputIndexReview';

import {
  digest,
  running,
  validateImportPlan,
  validateSession,
  validateImportCopyReceipt,
} from '../../library/importPlanValidation';

export function ImportPlanReview({
  manifest,
  comparison,
  initialPlan,
  onActivityChange,
}: {
  manifest: CopyManifest;
  comparison?: ImportComparisonJob | undefined;
  initialPlan?: ImportPlan;
  onActivityChange?: (active: boolean) => void;
}) {
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [plan, setPlan] = useState<ImportPlan | null>(initialPlan ?? null);
  const [history, setHistory] = useState<ImportHistory | null>(null);
  const [ack, setAck] = useState(false);
  const [approval, setApproval] = useState<ImportSkipApproval | null>(null);
  const [job, setJob] = useState<ImportSession | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyActive, setCopyActive] = useState(false);
  const [copyReceipts, setCopyReceipts] = useState<ImportCopyReceipt[]>([]);
  const [capability, setCapability] = useState<{ available: boolean; reason: string } | null>(
    comparison ? { available: true, reason: '' } : null,
  );
  const epoch = useRef(0);
  const retained = useRef<string | null>(null);
  const jobId = job?.id;
  const active = job !== null && running(job);
  useEffect(() => {
    onActivityChange?.(pending || active || copyActive);
    return () => onActivityChange?.(false);
  }, [pending, active, copyActive, onActivityChange]);
  const bundles =
    comparison?.state === 'completed' && comparison.report?.allScopeAvailable
      ? comparison.report.bundles
      : null;
  useEffect(() => {
    if (comparison) return;
    let live = true;
    void libraryData({ kind: 'organization_copy_capabilities' }, 'organization_copy_capabilities')
      .then((value) => {
        if (live) setCapability(value);
      })
      .catch((error) => {
        if (live) setCapability({ available: false, reason: errorMessage(error) });
      });
    return () => {
      live = false;
    };
  }, [comparison]);
  useEffect(
    () => () => {
      epoch.current += 1;
      if (retained.current)
        void library({
          kind: 'organization_import_session_discard',
          plan_id: manifest.planId,
          session_id: retained.current,
        }).catch(() => {});
    },
    [manifest.planId],
  );
  useEffect(
    () => () => {
      if (plan)
        void library({
          kind: 'organization_import_plan_release',
          plan_id: manifest.planId,
          import_plan_id: plan.id,
        }).catch(() => {});
    },
    [plan, manifest.planId],
  );
  useEffect(() => {
    if (!jobId || !active || !plan) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const next = await libraryData(
          { kind: 'organization_import_session', plan_id: manifest.planId, session_id: jobId! },
          'organization_import_session',
        );
        if (!live) return;
        validateSession(next, plan!, jobId);
        setJob(next);
        if (running(next)) timer = setTimeout(() => void poll(), 250);
      } catch (error) {
        if (live) {
          setError(errorMessage(error));
          timer = setTimeout(() => void poll(), 1000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 100);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [jobId, active, plan, manifest.planId]);
  async function work(action: (token: number) => Promise<void>) {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      await action(token);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  async function save(token: number) {
    if (!bundles || !comparison) return;
    const selected: ImportBundleChoice[] = bundles.map((bundle) => ({
      fileId: bundle.fileIds[0]!,
      existingGroupId: choices[bundle.fileIds[0]!] === 'copy' ? null : choices[bundle.fileIds[0]!]!,
    }));
    const next = await libraryData(
      {
        kind: 'organization_import_plan_create',
        input: { comparisonJobId: comparison.id, choices: selected },
      },
      'organization_import_plan',
    );
    validateImportPlan(next, manifest, comparison);
    if (epoch.current === token) {
      setPlan(next);
      setCopyReceipts([]);
      setApproval(null);
      setJob(null);
      setAck(false);
    }
  }
  async function readHistory(token: number) {
    const value = await libraryData(
      { kind: 'organization_import_history', plan_id: manifest.planId, manifest_id: manifest.id },
      'organization_import_history',
    );
    if (
      value.plans.length > 32 ||
      value.sessions.length > 32 ||
      new Set(value.plans.map((plan) => plan.id)).size !== value.plans.length ||
      value.plans.some(
        (plan) =>
          plan.manifestId !== manifest.id ||
          !digest(plan.digest) ||
          !Number.isInteger(plan.copyFiles) ||
          !Number.isInteger(plan.skipFiles) ||
          plan.copyFiles < 0 ||
          plan.skipFiles < 0 ||
          plan.copyFiles + plan.skipFiles < 1 ||
          plan.copyFiles + plan.skipFiles > 512,
      ) ||
      value.sessions.some(
        (job) =>
          job.planId !== manifest.planId ||
          !digest(job.digest) ||
          job.outputCreated ||
          !job.sourcesRetained,
      )
    )
      throw new Error('Import history does not match this confirmed manifest.');
    const sourcePlans = value.plans.filter(
      (saved) => !saved.sourceRootId || saved.sourceRootId === manifest.files[0]?.rootId,
    );
    if (sourcePlans.some((saved) => saved.copyFiles + saved.skipFiles !== manifest.files.length))
      throw new Error('Import history does not match this source library.');
    const ids = new Set(sourcePlans.map((saved) => saved.id));
    const sourceHistory = {
      plans: sourcePlans,
      sessions: value.sessions.filter((session) => ids.has(session.importPlanId)),
    };
    const receipts: ImportCopyReceipt[] = [];
    if (plan?.copyFileIds.length) {
      const status = await libraryData({ kind: 'status' }, 'status');
      const jobs = status.copyJobs ?? [];
      if (jobs.length > 32) throw new Error('Copy job history exceeds its review bound.');
      for (const job of jobs.filter(
        (job) => job.planId === manifest.planId && job.state === 'completed_verified',
      )) {
        const receipt = await libraryData(
          { kind: 'organization_import_copy_receipt', job_id: job.id },
          'organization_import_copy_receipt',
        );
        if (receipt?.importPlanId === plan.id) {
          validateImportCopyReceipt(receipt, plan, job);
          receipts.push(receipt);
        }
      }
    }
    if (epoch.current === token) {
      setHistory(sourceHistory);
      setCopyReceipts(receipts);
    }
  }
  async function open(token: number, id: string, session?: ImportSession) {
    const next = await libraryData(
      { kind: 'organization_import_plan_open', plan_id: manifest.planId, import_plan_id: id },
      'organization_import_plan',
    );
    validateImportPlan(next, manifest);
    if (next.id !== id) throw new Error('A different import plan was returned.');
    if (session) validateSession(session, next);
    if (epoch.current === token) {
      setPlan(next);
      setCopyReceipts([]);
      setApproval(null);
      setAck(false);
      setJob(session ?? null);
      retained.current = session?.id ?? null;
    }
  }
  async function approve(token: number) {
    if (!plan) return;
    setApproval(null);
    const value = await libraryData(
      {
        kind: 'organization_import_skip_approve',
        input: {
          planId: manifest.planId,
          importPlanId: plan.id,
          digest: plan.digest,
          sourcesRetainedAcknowledged: ack,
        },
      },
      'organization_import_skip_approval',
    );
    if (
      value.importPlanId !== plan.id ||
      value.digest !== plan.digest ||
      !value.id ||
      value.expiresInSeconds > 600
    )
      throw new Error('The approval belongs to different import decisions.');
    if (epoch.current === token) setApproval(value);
    else
      void library({
        kind: 'organization_import_plan_release',
        plan_id: manifest.planId,
        import_plan_id: plan.id,
      }).catch(() => {});
  }
  async function start(token: number) {
    if (!plan || !approval) return;
    setJob(null);
    if (retained.current) {
      await library({
        kind: 'organization_import_session_discard',
        plan_id: manifest.planId,
        session_id: retained.current,
      });
      retained.current = null;
    }
    const input = {
      planId: manifest.planId,
      importPlanId: plan.id,
      digest: plan.digest,
      approvalId: approval.id,
    };
    setApproval(null);
    const value = await libraryData(
      { kind: 'organization_import_skip_start', input },
      'organization_import_session',
    );
    validateSession(value, plan);
    if (epoch.current !== token) {
      void library({
        kind: 'organization_import_session_discard',
        plan_id: manifest.planId,
        session_id: value.id,
      }).catch(() => {});
      return;
    }
    retained.current = value.id;
    setJob(value);
  }
  async function cancel(token: number) {
    if (!job || !plan) return;
    const value = await libraryData(
      { kind: 'organization_import_session_cancel', plan_id: manifest.planId, session_id: job.id },
      'organization_import_session',
    );
    validateSession(value, plan, job.id);
    if (epoch.current === token) setJob(value);
  }
  return (
    <chakra.section
      aria-label="Import decisions"
      className="organization-destination"
      css={{
        margin: '24px 0',
        padding: '16px',
        border: '1px solid {colors.border}',
        borderRadius: 'var(--control-radius)',
        '& h3': { marginTop: '0' },
        '& bdi': { whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' },
      }}
    >
      <Heading as="h4" unstyled>
        Import decisions
      </Heading>
      <Text>
        Save one explicit choice per confirmed bundle. Source files remain in place. A saved plan
        grants no approval.
      </Text>
      {bundles && (
        <chakra.fieldset disabled={pending || active || copyActive}>
          <chakra.legend>Choose each whole bundle</chakra.legend>
          {bundles.map((bundle) => (
            <Field.Root key={bundle.fileIds[0]} asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>
                    <bdi>
                      {manifest.files.find((file) => file.id === bundle.fileIds[0])?.sourceName}
                    </bdi>{' '}
                    · {bundle.fileIds.length} members
                  </span>
                </Field.Label>
                <NativeSelect
                  aria-label={`Import choice for ${manifest.files.find((file) => file.id === bundle.fileIds[0])?.sourceName}`}
                  value={choices[bundle.fileIds[0]!] ?? ''}
                  onChange={(event) => {
                    setChoices((old) => ({ ...old, [bundle.fileIds[0]!]: event.target.value }));
                    setPlan(null);
                    setApproval(null);
                    setAck(false);
                    setJob(null);
                  }}
                >
                  <option value="">Choose an action</option>
                  <option value="copy">Copy every selected member</option>
                  {bundle.existingGroups.map((group, index) => (
                    <option key={group.id} value={group.id}>
                      Review existing group {index + 1} · {group.members.length} members
                    </option>
                  ))}
                </NativeSelect>
              </label>
            </Field.Root>
          ))}
          <Button
            type="button"
            disabled={bundles.some((bundle) => !choices[bundle.fileIds[0]!])}
            onClick={() => void work(save)}
          >
            Save import decisions
          </Button>
        </chakra.fieldset>
      )}
      {!capability?.available && (
        <Text>{capability?.reason ?? 'Checking import availability…'}</Text>
      )}
      <Button
        type="button"
        disabled={pending || active || copyActive || capability?.available !== true}
        onClick={() => void work(readHistory)}
      >
        Review saved import decisions
      </Button>
      {history && (
        <Box>
          <Text>Recent import plans and attempts (up to 32 each).</Text>
          {history.plans.map((saved) => (
            <Text key={saved.id}>
              <Button
                type="button"
                disabled={pending || active || copyActive}
                onClick={() => void work((token) => open(token, saved.id))}
              >
                Open import plan: {saved.copyFiles} copy / {saved.skipFiles} existing ·{' '}
                {saved.createdAt}
              </Button>
            </Text>
          ))}
          {history.sessions.map((session) => (
            <Text key={session.id}>
              <Button
                type="button"
                disabled={pending || active || copyActive}
                onClick={() => void work((token) => open(token, session.importPlanId, session))}
              >
                Review import attempt: {session.state.replaceAll('_', ' ')} · {session.createdAt}
              </Button>
            </Text>
          ))}
          {!history.plans.length && (
            <Text>No import decisions are retained for this manifest.</Text>
          )}
          {copyReceipts.map((receipt) => (
            <Box key={receipt.jobId}>
              <Text>
                Completed import: {receipt.copiedFileIds.length} new{' '}
                {receipt.copiedFileIds.length === 1 ? 'file' : 'files'} copied /{' '}
                {receipt.skippedFiles.length} existing{' '}
                {receipt.skippedFiles.length === 1 ? 'file' : 'files'} verified ·{' '}
                {receipt.verifiedAt}. Sources retained; no approval restored.
              </Text>
              <ImportOutputIndexReview receipt={receipt} sourceRootId={manifest.files[0]!.rootId} />
            </Box>
          ))}
        </Box>
      )}
      {plan && (
        <>
          <Text role="status">
            Saved import plan: {plan.copyFileIds.length} files to copy · {plan.skipFiles.length}{' '}
            existing files selected for fresh verification.
          </Text>
          <details>
            <summary>Saved file decisions</summary>
            <List.Root unstyled>
              {manifest.files.map((file) => {
                const keeper = plan.skipFiles.find((keeper) => keeper.fileId === file.id);
                return (
                  <List.Item unstyled key={file.id}>
                    <bdi>{file.sourceName}</bdi> →{' '}
                    {keeper ? (
                      <>
                        <bdi>{keeper.keeperRelativePath}</bdi>
                        {keeper.sameNativeObject
                          ? ' · same native object; no independent copy'
                          : ''}
                      </>
                    ) : (
                      <>
                        <bdi>{file.targetRelativePath}</bdi> · planned new copy
                      </>
                    )}
                  </List.Item>
                );
              })}
            </List.Root>
          </details>
          {plan.executionAvailable ? (
            <>
              <label>
                <input
                  type="checkbox"
                  checked={ack}
                  disabled={pending || active || copyActive || capability?.available !== true}
                  onChange={(event) => {
                    setAck(event.target.checked);
                    setApproval(null);
                    if (!event.target.checked)
                      void library({
                        kind: 'organization_import_plan_release',
                        plan_id: manifest.planId,
                        import_plan_id: plan.id,
                      }).catch(() => {});
                  }}
                />
                I understand sources remain and every chosen existing file must be freshly
                byte-compared.
              </label>
              <Button
                type="button"
                disabled={!ack || pending || active || copyActive || capability?.available !== true}
                onClick={() => void work(approve)}
              >
                Approve existing-bundle import
              </Button>
              {approval && (
                <Button
                  type="button"
                  disabled={pending || active || copyActive || capability?.available !== true}
                  onClick={() => void work(start)}
                >
                  Verify and record skipped import
                </Button>
              )}
            </>
          ) : capability?.available === true ? (
            <VerifiedCopyWorkflow
              key={plan.id}
              manifest={manifest}
              importPlan={plan}
              onActivityChange={setCopyActive}
            />
          ) : (
            <Text>Verified import copying is unavailable in this build.</Text>
          )}
        </>
      )}
      {job && (
        <Text role="status">
          Import verification: {job.state.replaceAll('_', ' ')} · {job.verifiedFiles}/
          {job.totalFiles} complete files · {BigInt(job.bytesRead).toLocaleString()} payload bytes
          read.
        </Text>
      )}
      {active && (
        <>
          <Button
            type="button"
            disabled={pending || job.state === 'cancel_requested'}
            onClick={() => void work(cancel)}
          >
            Cancel import verification
          </Button>
          <CopyIoControls />
        </>
      )}
      {job?.state === 'completed' && (
        <Text>
          Import recorded: {job.verifiedFiles} existing files freshly compared. Source files
          retained; no output folder created. Observed {job.finishedAt}.
        </Text>
      )}
      {job?.state === 'needs_recovery' && (
        <Text>
          This retained attempt has no complete outcome. It will not replay. Review current files
          and obtain fresh approval for another verification attempt.
        </Text>
      )}
      {job?.state === 'canceled' && (
        <Text>Import canceled; no skipped-import completion was recorded.</Text>
      )}
      {(error || job?.error) && <Text role="alert">{error ?? errorMessage(job?.error)}</Text>}
    </chakra.section>
  );
}
