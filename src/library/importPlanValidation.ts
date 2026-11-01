import type {
  CopyManifest,
  CopyActionReview,
  CopyWorkflowJob,
  ImportCopyReceipt,
  ImportComparisonJob,
  ImportPlan,
  ImportSession,
} from '../bindings';

export function validateImportCopyReview(
  review: CopyActionReview,
  manifest: CopyManifest,
  plan: ImportPlan,
) {
  validateImportPlan(plan, manifest);
  const expected = manifest.files.filter((file) => plan.copyFileIds.includes(file.id));
  if (
    !expected.length ||
    review.sourceRootId !== plan.sourceRootId ||
    review.planId !== plan.planId ||
    review.manifestId !== manifest.id ||
    review.assessment.manifestDigest !== manifest.digest ||
    expected.length !== review.files.length ||
    expected.some((file, index) => {
      const actual = review.files[index];
      return (
        !actual ||
        actual.id !== file.id ||
        actual.bytes !== file.bytes ||
        actual.targetPath !== `${review.outputFolder}/${file.targetRelativePath}`
      );
    }) ||
    review.assessment.capacity.logicalBytes !==
      expected.reduce((total, file) => total + BigInt(file.bytes), 0n).toString()
  )
    throw new Error('The import copy review differs from the exact saved new-file decisions.');
}
export function validateImportCopyReceipt(
  receipt: ImportCopyReceipt,
  plan: ImportPlan,
  job: CopyWorkflowJob,
) {
  if (
    job.state !== 'completed_verified' ||
    receipt.jobId !== job.id ||
    receipt.copyReviewId !== job.reviewId ||
    receipt.planId !== plan.planId ||
    receipt.manifestId !== plan.manifestId ||
    receipt.importPlanId !== plan.id ||
    receipt.importDigest !== plan.digest ||
    !receipt.sourcesRetained ||
    !receipt.verifiedAt ||
    !/^\d{1,39}$/.test(receipt.keeperBytesRead) ||
    receipt.copiedFileIds.length !== job.totalFiles ||
    JSON.stringify(receipt.copiedFileIds) !== JSON.stringify(plan.copyFileIds) ||
    JSON.stringify(receipt.skippedFiles) !== JSON.stringify(plan.skipFiles)
  )
    throw new Error('The completed import receipt does not match this job and saved decisions.');
}

export const digest = (value: string) => /^[a-f0-9]{64}$/.test(value);
export const running = (job: ImportSession) =>
  ['queued', 'running', 'cancel_requested'].includes(job.state);
function groups(manifest: CopyManifest) {
  const remaining = new Set(manifest.files.map((file) => file.id));
  const result: string[][] = [];
  for (const first of manifest.files) {
    if (!remaining.delete(first.id)) continue;
    const members = [first.id];
    const bundles = new Set(first.bundleAssetIds);
    let changed = true;
    while (changed) {
      changed = false;
      for (const file of manifest.files) {
        if (remaining.has(file.id) && file.bundleAssetIds.some((id) => bundles.has(id))) {
          remaining.delete(file.id);
          members.push(file.id);
          file.bundleAssetIds.forEach((id) => bundles.add(id));
          changed = true;
        }
      }
    }
    result.push(manifest.files.filter((file) => members.includes(file.id)).map((file) => file.id));
  }
  return result;
}
export function validateImportPlan(
  plan: ImportPlan,
  manifest: CopyManifest,
  comparison?: ImportComparisonJob,
) {
  const bundles = groups(manifest);
  const covered = [...plan.copyFileIds, ...plan.skipFiles.map((file) => file.fileId)];
  if (
    !plan.id ||
    plan.planId !== manifest.planId ||
    plan.manifestId !== manifest.id ||
    plan.manifestDigest !== manifest.digest ||
    !digest(plan.digest) ||
    !plan.sourcesRetained ||
    plan.sourceRootId !== manifest.files[0]?.rootId ||
    plan.executionAvailable !== (plan.copyFileIds.length === 0) ||
    covered.length !== manifest.files.length ||
    new Set(covered).size !== covered.length ||
    covered.some((id) => !manifest.files.some((file) => file.id === id)) ||
    plan.scopeRootIds.length === 0 ||
    plan.scopeRootIds.length > 16 ||
    new Set(plan.scopeRootIds).size !== plan.scopeRootIds.length ||
    plan.scopeRootIds.includes(plan.sourceRootId) ||
    plan.choices.length !== bundles.length ||
    new Set(plan.choices.map((choice) => choice.fileId)).size !== bundles.length ||
    plan.skipFiles.some(
      (file) =>
        !plan.scopeRootIds.includes(file.keeperRootId) ||
        !file.keeperAssetId ||
        !digest(file.digest),
    ) ||
    plan.choices.some((choice) => {
      const bundle = bundles.find((files) => files[0] === choice.fileId);
      if (!bundle) return true;
      if (choice.existingGroupId === null)
        return !bundle.every((id) => plan.copyFileIds.includes(id));
      if (
        !digest(choice.existingGroupId) ||
        !bundle.every((id) => plan.skipFiles.some((file) => file.fileId === id))
      )
        return true;
      const keepers = plan.skipFiles.filter((file) => bundle.includes(file.fileId));
      if (
        new Set(keepers.map((file) => file.keeperRootId)).size !== 1 ||
        new Set(keepers.map((file) => file.keeperAssetId)).size !== bundle.length
      )
        return true;
      if (comparison?.report) {
        const observed = comparison.report.bundles
          .find((item) => item.fileIds[0] === choice.fileId)
          ?.existingGroups.find((group) => group.id === choice.existingGroupId);
        if (
          !observed ||
          observed.members.length !== bundle.length ||
          keepers.some(
            (keeper) =>
              keeper.keeperRootId !== observed.rootId ||
              !observed.members.some(
                (member) =>
                  member.fileId === keeper.fileId && member.assetId === keeper.keeperAssetId,
              ),
          )
        )
          return true;
      }
      return false;
    }) ||
    (comparison &&
      (comparison.manifestId !== plan.manifestId ||
        JSON.stringify([...comparison.scopeRootIds].sort()) !==
          JSON.stringify([...plan.scopeRootIds].sort())))
  )
    throw new Error('The saved import decisions do not match the complete confirmed bundles.');
}
export function validateSession(job: ImportSession, plan: ImportPlan, id?: string) {
  if (
    !job.id ||
    (id && job.id !== id) ||
    job.planId !== plan.planId ||
    job.importPlanId !== plan.id ||
    job.digest !== plan.digest ||
    job.totalFiles !== plan.skipFiles.length ||
    job.verifiedFiles > job.totalFiles ||
    job.verifiedFiles < 0 ||
    !/^\d{1,39}$/.test(job.bytesRead) ||
    !job.sourcesRetained ||
    job.outputCreated ||
    ![
      'queued',
      'running',
      'cancel_requested',
      'completed',
      'canceled',
      'failed',
      'needs_recovery',
    ].includes(job.state) ||
    (job.state === 'completed' &&
      (job.verifiedFiles !== job.totalFiles || job.error || !job.finishedAt))
  )
    throw new Error('The import session does not match its saved decisions.');
}
