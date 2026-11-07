import { Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type { CopyManifest, ImportPlan, ImportPlanSummary } from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { ImportPlanReview } from '@/components/ImportPlanReview';
import { digest, validateImportPlan, importSourceScope } from '../../library/importPlanValidation';

export function SavedImportPlans({ planId }: { planId: string }) {
  const [plans, setPlans] = useState<ImportPlanSummary[] | null>(null);
  const [opened, setOpened] = useState<{
    plan: ImportPlan;
    manifest: CopyManifest;
    sourceLabel: string | null;
  } | null>(null);
  const [pending, setPending] = useState(false);
  const [unavailable, setUnavailable] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  async function run(action: (token: number) => Promise<void>) {
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
  async function list(token: number) {
    const capability = await libraryData(
      { kind: 'organization_copy_capabilities' },
      'organization_copy_capabilities',
    );
    if (epoch.current !== token) return;
    if (!capability.available) {
      setUnavailable(capability.reason);
      return;
    }
    const rows = await libraryData(
      { kind: 'organization_import_saved_plans', plan_id: planId },
      'organization_import_saved_plans',
    );
    if (
      rows.length > 32 ||
      new Set(rows.map((plan) => plan.id)).size !== rows.length ||
      rows.some(
        (plan) =>
          !plan.id ||
          !plan.manifestId ||
          !digest(plan.digest) ||
          !Number.isInteger(plan.copyFiles) ||
          !Number.isInteger(plan.skipFiles) ||
          plan.copyFiles < 0 ||
          plan.skipFiles < 0 ||
          plan.copyFiles + plan.skipFiles < 1 ||
          plan.copyFiles + plan.skipFiles > 512,
      )
    )
      throw new Error('Retained import summaries are invalid.');
    if (epoch.current === token) setPlans(rows);
  }
  async function open(token: number, summary: ImportPlanSummary) {
    const plan = await libraryData(
      { kind: 'organization_import_plan_open', plan_id: planId, import_plan_id: summary.id },
      'organization_import_plan',
    );
    if (
      plan.planId !== planId ||
      plan.id !== summary.id ||
      plan.digest !== summary.digest ||
      plan.manifestId !== summary.manifestId ||
      (summary.sourceRootId && plan.sourceRootId !== summary.sourceRootId)
    )
      throw new Error('A different retained import plan was returned.');
    const manifest = await libraryData(
      { kind: 'organization_copy_manifest_open', plan_id: planId, manifest_id: summary.manifestId },
      'organization_copy_manifest',
    );
    const source = importSourceScope(manifest, plan.sourceRootId);
    validateImportPlan(plan, source);
    let sourceLabel: string | null = null;
    if (new Set(manifest.files.map((file) => file.rootId)).size > 1) {
      const status = await libraryData({ kind: 'status' }, 'status');
      sourceLabel =
        status.roots.find((root) => root.id === plan.sourceRootId)?.label ??
        'Unavailable source library';
    }
    if (epoch.current === token) setOpened({ plan, manifest: source, sourceLabel });
  }
  return (
    <chakra.section aria-label="Retained imports" className="organization-destination">
      <Heading as="h4" unstyled>
        Retained imports
      </Heading>
      <Text>
        Review saved import decisions and receipts without creating a new companion decision or
        restoring approval. Any new attempt checks current files and needs fresh approval.
      </Text>
      <Button
        type="button"
        disabled={pending || unavailable !== null}
        onClick={() => void run(list)}
      >
        {plans ? 'Refresh retained imports' : 'Check retained import history'}
      </Button>
      {unavailable && <Text>{unavailable}</Text>}
      {plans?.map((plan) => (
        <Text key={plan.id}>
          <Button
            type="button"
            disabled={pending}
            onClick={() => void run((token) => open(token, plan))}
          >
            Review retained import: {plan.copyFiles} copy / {plan.skipFiles} existing ·{' '}
            {plan.createdAt}
          </Button>
        </Text>
      ))}
      {plans?.length === 0 && <Text>No retained imports for this saved draft.</Text>}
      {opened && (
        <>
          {opened.sourceLabel && (
            <Text>
              Source library: <bdi>{opened.sourceLabel}</bdi> · {opened.manifest.files.length} files
              in this retained import.
            </Text>
          )}
          <ImportPlanReview
            key={opened.plan.id}
            manifest={opened.manifest}
            initialPlan={opened.plan}
          />
        </>
      )}
      {error && <Text role="alert">{error}</Text>}
    </chakra.section>
  );
}
