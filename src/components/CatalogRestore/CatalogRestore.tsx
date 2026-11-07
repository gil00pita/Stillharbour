import { Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  CatalogRestoreApproval,
  CatalogRestorePrepared,
  CatalogRestoreReview,
  SnapshotJob,
} from '../../bindings';
import { errorMessage, pickDirectory } from '../../ipc';
import { snapshotRequest } from '../../backup/snapshotIpc';
import { catalogSwitchStore } from '../../backup/catalogSwitchState';

export function CatalogRestore({
  job,
  catalogId,
  disabled,
  onActivityChange,
}: {
  job: SnapshotJob;
  catalogId: string;
  disabled: boolean;
  onActivityChange: (jobId: string, active: boolean) => void;
}) {
  if (job.action !== 'restore') return null;
  if (
    job.state !== 'completed_verified' ||
    !job.snapshotId ||
    !job.included ||
    !job.structuralChecked ||
    !job.dataReadVerified ||
    !job.restoredCompared ||
    !job.sourceRetained
  ) {
    return (
      <Text>
        Catalog switch review requires a completed whole-snapshot restore with full verification.
      </Text>
    );
  }
  return (
    <CatalogRestoreWorkspace
      key={`${catalogId}:${job.id}:${job.snapshotId}`}
      source={{
        kind: 'verified_snapshot_restore',
        restoreJobId: job.id,
        snapshotId: job.snapshotId,
      }}
      catalogId={catalogId}
      disabled={disabled}
      onActivityChange={onActivityChange}
    />
  );
}

type CatalogSource =
  | { kind: 'verified_snapshot_restore'; restoreJobId: string; snapshotId: string }
  | { kind: 'retained_pre_restore'; transactionId: string };

export function RetainedCatalogRestore({
  transactionId,
  catalogId,
  disabled,
  onActivityChange,
}: {
  transactionId: string;
  catalogId: string;
  disabled: boolean;
  onActivityChange: (scopeId: string, active: boolean) => void;
}) {
  return (
    <CatalogRestoreWorkspace
      key={`${catalogId}:retained:${transactionId}`}
      source={{ kind: 'retained_pre_restore', transactionId }}
      catalogId={catalogId}
      disabled={disabled}
      onActivityChange={onActivityChange}
    />
  );
}

function CatalogRestoreWorkspace({
  source,
  catalogId,
  disabled,
  onActivityChange,
}: {
  source: CatalogSource;
  catalogId: string;
  disabled: boolean;
  onActivityChange: (scopeId: string, active: boolean) => void;
}) {
  const activityKey =
    source.kind === 'verified_snapshot_restore'
      ? source.restoreJobId
      : `retained:${source.transactionId}`;
  const retained = source.kind === 'retained_pre_restore';
  const [review, setReview] = useState<CatalogRestoreReview | null>(null);
  const [approval, setApproval] = useState<CatalogRestoreApproval | null>(null);
  const [prepared, setPrepared] = useState<CatalogRestorePrepared | null>(null);
  const [acknowledged, setAcknowledged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [uncertain, setUncertain] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState('');
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      onActivityChange(activityKey, false);
    };
  }, [activityKey, onActivityChange]);
  useEffect(() => {
    onActivityChange(activityKey, busy || Boolean(prepared) || uncertain);
  }, [activityKey, busy, prepared, uncertain, onActivityChange]);

  function validate(value: CatalogRestoreReview) {
    const sourceMatches =
      value.sourceKind === source.kind &&
      (source.kind === 'verified_snapshot_restore'
        ? value.restoreJobId === source.restoreJobId &&
          value.snapshotId === source.snapshotId &&
          value.retainedTransactionId === null
        : value.restoreJobId === null &&
          value.snapshotId === null &&
          value.retainedTransactionId === source.transactionId);
    if (
      !sourceMatches ||
      value.catalogId !== catalogId ||
      !value.id ||
      !/^[a-fA-F0-9]{64}$/.test(value.digest) ||
      !value.currentRevision ||
      !value.restoredRevision ||
      new Set(value.roots.map((root) => root.rootId)).size !== value.roots.length ||
      value.roots.some((root) => !root.rootId || (root.confirmed && !root.confirmedDisplayPath))
    ) {
      throw new Error('Catalog review identity or root mapping changed. Request a fresh review.');
    }
  }
  async function action(message: string, operation: () => Promise<void>) {
    if (disabled || pending.current || prepared || uncertain) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    setProgress(message);
    try {
      await operation();
    } catch (cause) {
      if (alive.current) setError(errorMessage(cause));
    } finally {
      if (alive.current) {
        pending.current = false;
        setBusy(false);
        setProgress('');
      }
    }
  }
  const blocked = disabled || busy || Boolean(prepared) || uncertain;
  const ready = review?.approvalAvailable && review.roots.every((root) => root.confirmed);
  return (
    <chakra.section
      className="backup-review"
      css={{
        border: '1px solid {colors.border}',
        borderRadius: '8px',
        padding: '20px',
        marginBlock: '20px',
        minWidth: '0',
        background: '{colors.bg.panel}',
        '& fieldset': {
          display: 'grid',
          gap: '16px',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          padding: '16px',
          marginBlock: '20px',
        },
        '& legend': { fontWeight: '600', paddingInline: '6px' },
        '& label': { display: 'grid', gap: '8px', minWidth: '0' },
        "& label:has(> input[type='checkbox'])": {
          display: 'flex',
          alignItems: 'flex-start',
          gap: '8px',
        },
        "& input[type='checkbox']": { flex: '0 0 auto', marginTop: '4px' },
        "& input:not([type='checkbox'])": {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
        },
        '& textarea': {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
          resize: 'vertical',
          minHeight: '100px',
        },
        '& select': {
          width: '100%',
          boxSizing: 'border-box',
          minWidth: '0',
          border: '1px solid {colors.border}',
          borderRadius: '6px',
          background: '{colors.bg}',
          color: 'inherit',
          padding: '8px 10px',
          font: 'inherit',
        },
        '& .backup-form-grid': { gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' },
        '& small': { color: '{colors.fg.muted}' },
        '& .backup-table': { maxHeight: '360px' },
        '@media (max-width: 850px)': { '& .backup-form-grid': { gridTemplateColumns: '1fr' } },
      }}
      aria-label={retained ? 'Retained catalog switch' : 'Restored catalog switch'}
      data-catalog-source={review?.sourceKind}
      data-restore-job-id={review?.restoreJobId ?? undefined}
      data-snapshot-id={review?.snapshotId ?? undefined}
      data-retained-transaction-id={review?.retainedTransactionId ?? undefined}
    >
      <Heading as="h5" unstyled>
        {retained ? 'Use the catalog saved before the latest switch' : 'Use this restored catalog'}
      </Heading>
      {retained && (
        <Text>
          This local retained source comes from the latest activated catalog switch. Current edits
          will be preserved again in a new transaction; it does not need repository credentials or
          backup helpers.
        </Text>
      )}
      <Text>
        Review switching Stillharbour to this {retained ? 'retained' : 'restored'} catalog. The
        current catalog and its newer edits must be preserved separately; edits are not merged.
      </Text>
      {!prepared && !uncertain && (
        <Button
          disabled={blocked}
          onClick={() =>
            void action('Inspecting current and selected catalogs…', async () => {
              setApproval(null);
              setAcknowledged(false);
              setReview(null);
              const value = await snapshotRequest(
                source.kind === 'verified_snapshot_restore'
                  ? { kind: 'catalog_restore_review', restore_job_id: source.restoreJobId }
                  : {
                      kind: 'catalog_restore_review_retained',
                      transaction_id: source.transactionId,
                    },
                'catalog_restore_review',
              );
              if (!alive.current) return;
              validate(value);
              setReview(value);
            })
          }
        >
          {retained
            ? review
              ? 'Refresh retained catalog review'
              : 'Review latest retained catalog'
            : review
              ? 'Refresh catalog switch review'
              : 'Review catalog switch'}
        </Button>
      )}
      {review && (
        <>
          <Text>
            {review.sourceKind === 'retained_pre_restore' ? (
              <>
                Source: Local catalog saved before switch{' '}
                <code>{review.retainedTransactionId}</code>
              </>
            ) : (
              <>
                Source: Verified snapshot <code>{review.snapshotId}</code> restored by job{' '}
                <code>{review.restoreJobId}</code>
              </>
            )}
          </Text>
          <chakra.dl
            className="backup-evidence"
            css={{
              display: 'grid',
              gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
              gap: '12px 24px',
              '& dt': { color: '{colors.fg.muted}' },
              '& dd': { margin: '2px 0 0', overflowWrap: 'anywhere' },
              '@media (max-width: 850px)': { gridTemplateColumns: '1fr' },
            }}
          >
            <Box>
              <dt>
                {retained
                  ? 'Current and retained catalog UUID'
                  : 'Current and restored catalog UUID'}
              </dt>
              <dd>
                <code>{review.catalogId}</code>
              </dd>
            </Box>
            <Box>
              <dt>Validated catalog schema</dt>
              <dd>{review.schema}</dd>
            </Box>
            <Box>
              <dt>Current catalog revision</dt>
              <dd>
                <code>{review.currentRevision}</code>
              </dd>
            </Box>
            <Box>
              <dt>{retained ? 'Retained catalog revision' : 'Restored catalog revision'}</dt>
              <dd>
                <code>{review.restoredRevision}</code>
              </dd>
            </Box>
          </chakra.dl>
          <Text>
            Exact review digest: <code>{review.digest}</code>
          </Text>
          <Text>{review.warning}</Text>
          <Text>
            Each source root requires a fresh native selection matching its saved identity and
            volume. Displayed paths do not authorize access.
          </Text>
          <List.Root unstyled aria-label="Restored catalog root mappings">
            {review.roots.map((root) => (
              <List.Item unstyled key={root.rootId}>
                <strong>{root.label}</strong> · Root UUID <code>{root.rootId}</code>
                <Text
                  className="backup-path"
                  css={{ overflowWrap: 'anywhere', whiteSpace: 'pre-wrap' }}
                >
                  Saved location: {root.savedDisplayPath}
                </Text>
                <Text
                  className="backup-path"
                  css={{ overflowWrap: 'anywhere', whiteSpace: 'pre-wrap' }}
                >
                  {root.confirmed
                    ? `Native identity confirmed: ${root.confirmedDisplayPath}`
                    : 'Native identity not confirmed'}
                </Text>
                <Button
                  disabled={blocked}
                  onClick={() =>
                    void action(
                      'Choose the original source root in the native picker…',
                      async () => {
                        setApproval(null);
                        setAcknowledged(false);
                        const selection = await pickDirectory();
                        if (!alive.current || !selection) return;
                        const value = await snapshotRequest(
                          {
                            kind: 'catalog_restore_map',
                            review_id: review.id,
                            root_id: root.rootId,
                            selection_id: selection.id,
                          },
                          'catalog_restore_review',
                        );
                        if (!alive.current) return;
                        validate(value);
                        if (
                          JSON.stringify(value.roots.map((mapped) => mapped.rootId).sort()) !==
                            JSON.stringify(review.roots.map((mapped) => mapped.rootId).sort()) ||
                          value.restoredRevision !== review.restoredRevision ||
                          value.currentRevision !== review.currentRevision
                        ) {
                          throw new Error(
                            'Catalog restore scope changed during mapping. Request a fresh review.',
                          );
                        }
                        if (
                          !value.roots.some(
                            (mapped) => mapped.rootId === root.rootId && mapped.confirmed,
                          )
                        )
                          throw new Error('The selected root was not confirmed by the backend.');
                        setReview(value);
                      },
                    )
                  }
                >
                  Choose native location for {root.label}
                </Button>
              </List.Item>
            ))}
          </List.Root>
          {!ready && (
            <Text role="status">
              Catalog approval is unavailable until every required native mapping and backend check
              is confirmed.
            </Text>
          )}
          <label>
            <input
              type="checkbox"
              checked={acknowledged}
              disabled={blocked || Boolean(approval) || !ready}
              onChange={(event) => setAcknowledged(event.target.checked)}
            />
            I reviewed these exact catalog revisions and root mappings. Preserve my current catalog
            and newer edits separately; do not merge them.
          </label>
          {!approval ? (
            <Button
              disabled={blocked || !ready || !acknowledged}
              onClick={() =>
                void action('Approving this exact catalog switch…', async () => {
                  const value = await snapshotRequest(
                    {
                      kind: 'catalog_restore_approve',
                      review_id: review.id,
                      digest: review.digest,
                    },
                    'catalog_restore_approval',
                  );
                  if (!alive.current) return;
                  if (!value.id || value.reviewId !== review.id || value.digest !== review.digest)
                    throw new Error(
                      'Catalog approval does not match this review. Request a fresh review.',
                    );
                  setApproval(value);
                })
              }
            >
              Approve catalog switch
            </Button>
          ) : (
            <>
              <Text>Approval recorded. Catalog preparation has not started.</Text>
              <Button
                disabled={blocked}
                onClick={() =>
                  void action(
                    'Preparing the catalog switch and preserving current edits…',
                    async () => {
                      const approved = approval;
                      setApproval(null);
                      let ticket: number | undefined;
                      try {
                        ticket = catalogSwitchStore.begin({
                          catalogId,
                          sourceKind: review.sourceKind,
                          restoreJobId: review.restoreJobId,
                          snapshotId: review.snapshotId,
                          retainedTransactionId: review.retainedTransactionId,
                          reviewDigest: review.digest,
                        });
                        const value = await snapshotRequest(
                          { kind: 'catalog_restore_prepare', approval_id: approved.id },
                          'catalog_restore_prepared',
                        );
                        if (
                          !value.transactionId ||
                          value.catalogId !== catalogId ||
                          !value.restartRequired ||
                          !value.currentCatalogRetained
                        )
                          throw new Error(
                            'Catalog preparation did not confirm its required preservation and restart state.',
                          );
                        catalogSwitchStore.prepared(ticket, value);
                        if (alive.current) setPrepared(value);
                      } catch (cause) {
                        if (ticket !== undefined)
                          catalogSwitchStore.unconfirmed(ticket, errorMessage(cause));
                        if (alive.current) setUncertain(true);
                        throw cause;
                      }
                    },
                  )
                }
              >
                Prepare catalog switch
              </Button>
            </>
          )}
        </>
      )}
      {busy && <Text role="status">{progress}</Text>}
      {error && (
        <Text
          role="alert"
          className="error"
          css={{
            border: '1px solid {colors.border.control}',
            borderRadius: 'var(--control-radius)',
            background: '{colors.danger.bg}',
            color: '{colors.danger.fg}',
            padding: '16px 20px',
            margin: '24px 0 0',
            fontSize: '13px',
            lineHeight: '1.6',
            '& p': { margin: '0' },
            '& button': {
              marginTop: '12px',
              border: '1px solid currentColor',
              background: 'transparent',
              color: 'inherit',
              borderRadius: '4px',
              padding: '7px 12px',
            },
          }}
        >
          {error}
        </Text>
      )}
      {uncertain && (
        <Text role="status">
          Preparation was not confirmed. Close and reopen Stillharbour to reconcile the pending
          switch before another attempt. Existing catalogs are retained.
        </Text>
      )}
      {prepared && (
        <Box role="status">
          <Text>{prepared.message}</Text>
          <Text>
            Pending restart. The current catalog is retained; this response does not confirm that
            the restored catalog is active.
          </Text>
          <Text>
            Close and reopen Stillharbour to validate and apply the prepared catalog switch.
          </Text>
          <Text>
            Catalog switch transaction: <code>{prepared.transactionId}</code>
          </Text>
        </Box>
      )}
    </chakra.section>
  );
}
