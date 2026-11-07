import { Field, Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Textarea } from '@/components/Textarea';
import { useEffect, useRef, useState } from 'react';
import type {
  OrganizationPlanDetail,
  OrganizationPlanPage,
  PlanExport,
  PlanExportFormat,
  PlanExportSaved,
  OrganizationRow,
  BundleConfirmation,
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';
import { OrganizationTrees } from '@/components/OrganizationTrees';
import { BundleInventoryReview } from '@/components/BundleInventoryReview';
import { CopyManifestReview } from '@/components/CopyManifestReview';
import { SavedImportPlans } from '@/components/SavedImportPlans';

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
    <chakra.dialog
      className="organization-preview"
      css={{
        width: 'min(1100px, calc(100vw - 48px))',
        maxHeight: 'calc(100dvh - 48px)',
        padding: '0',
        border: '1px solid {colors.border}',
        borderRadius: '12px',
        color: 'inherit',
        background: '{colors.bg.panel}',
        '&[open]': { display: 'flex', flexDirection: 'column' },
        '&::backdrop': { background: 'rgb(0 0 0 / 65%)' },
        '@media (max-width: 760px)': {
          width: 'calc(100vw - 24px)',
          maxHeight: 'calc(100dvh - 24px)',
        },
        '&.organization-inline': {
          display: 'flex',
          flexDirection: 'column',
          position: 'static',
          width: '100%',
          maxHeight: 'none',
          margin: '0',
          border: '0',
          borderRadius: '0',
          background: 'transparent',
        },
      }}
      ref={dialog}
      aria-labelledby="saved-plans-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <chakra.header
        className="organization-heading"
        css={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '24px',
          padding: '20px 24px',
          flexShrink: '0',
          borderBottom: '1px solid {colors.border}',
          '& h2': { margin: '0', marginBottom: '8px', fontSize: '20px' },
          '& p': { margin: '0' },
        }}
      >
        <Heading as="h2" unstyled id="saved-plans-title">
          Saved organization plans
        </Heading>
        <Button onClick={onClose}>Close saved plans</Button>
      </chakra.header>
      <Box
        className="organization-body"
        css={{
          padding: '0 24px 24px',
          overflowY: 'auto',
          minHeight: '0',
          '& fieldset': {
            display: 'grid',
            gap: '16px',
            padding: '0',
            border: '0',
            margin: '24px 0',
            minWidth: '0',
          },
          '& label': { display: 'grid', gap: '8px', minWidth: '0' },
          '& input': { width: '100%', minWidth: '0' },
          '& select': { width: '100%', minWidth: '0' },
          '& fieldset > button': { justifySelf: 'start' },
          '& bdi': { whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' },
        }}
        aria-busy={busy}
      >
        <Text>
          Retained drafts preserve their original paths and template versions. Copying remains
          unavailable.
        </Text>
        <Button disabled={busy} onClick={() => void list()}>
          Refresh plans
        </Button>
        {error && (
          <Text
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
            role="alert"
          >
            {error}
          </Text>
        )}
        {page?.plans.length === 0 && (
          <Text>
            No saved plans yet. Select photos, build an organization preview and choose a
            destination.
          </Text>
        )}
        <List.Root unstyled>
          {page?.plans.map((plan) => (
            <List.Item unstyled key={plan.id}>
              <Button disabled={busy} onClick={() => void open(plan.id)}>
                <bdi>{plan.destinationPath}</bdi> · {plan.proposed} proposed · {plan.createdAt}
              </Button>
            </List.Item>
          ))}
        </List.Root>
        {page?.next && (
          <Button disabled={busy} onClick={() => void list(page.next)}>
            Load more plans
          </Button>
        )}
        {detail && (
          <chakra.section aria-label="Saved plan detail">
            <Heading as="h3" unstyled>
              Draft · {detail.plan.createdAt}
            </Heading>
            <Text>
              <bdi>{detail.plan.destinationPath}</bdi>
            </Text>
            {detail.stale && (
              <Text role="alert">
                The catalog changed. This retained draft needs a fresh preview before any future
                approval.
              </Text>
            )}
            <Text>
              Destination identity was checked when saved. It is not reserved or revalidated by
              reopening this draft.
            </Text>
            <Text>
              {detail.plan.preview.proposed} proposed · {detail.plan.preview.blocked} blocked ·{' '}
              {BigInt(detail.plan.preview.selectedBytes).toLocaleString()} selected file bytes
            </Text>
            {detail.plan.templateReference && (
              <Text>Template version {detail.plan.templateReference.version}</Text>
            )}
            <Box
              className="organization-table"
              css={{
                overflowX: 'auto',
                marginTop: '16px',
                '& table': { width: '100%', borderCollapse: 'collapse', tableLayout: 'fixed' },
                '& th': {
                  padding: '12px 16px 12px 0',
                  textAlign: 'left',
                  verticalAlign: 'top',
                  overflowWrap: 'anywhere',
                  borderBottom: '1px solid {colors.border}',
                },
                '& td': {
                  padding: '12px 16px 12px 0',
                  textAlign: 'left',
                  verticalAlign: 'top',
                  overflowWrap: 'anywhere',
                  borderBottom: '1px solid {colors.border}',
                },
                '& caption': {
                  textAlign: 'left',
                  paddingBottom: '12px',
                  color: '{colors.fg.muted}',
                },
                '& small': {
                  display: 'block',
                  color: '{colors.fg.muted}',
                  marginTop: '8px',
                  fontSize: '12px',
                  lineHeight: '16px',
                },
                '& bdi': { whiteSpace: 'pre-wrap' },
                '& p': { margin: '0 0 8px' },
              }}
            >
              <OrganizationTrees rows={detail.plan.preview.rows} />
              <Table.Root unstyled>
                <Table.Caption unstyled captionSide="top">
                  Retained source and destination paths
                </Table.Caption>
                <Table.Header unstyled>
                  <Table.Row unstyled>
                    <Table.ColumnHeader unstyled>Source</Table.ColumnHeader>
                    <Table.ColumnHeader unstyled>Destination relative path</Table.ColumnHeader>
                  </Table.Row>
                </Table.Header>
                <Table.Body unstyled>
                  {detail.plan.preview.rows.map((row) => (
                    <Table.Row unstyled key={row.assetId}>
                      <Table.Cell unstyled>
                        <bdi>{row.sourcePath}</bdi>
                        <Button
                          type="button"
                          disabled={busy || detail.stale || row.issues.length > 0}
                          onClick={(event) => {
                            companionOpener.current = event.currentTarget;
                            setCompanionRow(row);
                          }}
                        >
                          Review all companion filenames
                        </Button>
                        {companionChoices[row.assetId] && (
                          <Text
                            className="help"
                            css={{
                              fontSize: '13px',
                              color: '{colors.fg.muted}',
                              lineHeight: '20px',
                              margin: '8px 0',
                            }}
                          >
                            Chosen review: {companionChoices[row.assetId]?.id}
                          </Text>
                        )}
                      </Table.Cell>
                      <Table.Cell unstyled>
                        <bdi>{row.targetRelativePath ?? 'Blocked'}</bdi>
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table.Root>
            </Box>
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
            <SavedImportPlans key={`imports:${detail.plan.id}`} planId={detail.plan.id} />
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Copying requires fresh companion, destination and capacity checks, separate native
              preparation and explicit approval. Recovery and metadata fidelity limits apply.
            </Text>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Fingerprint: {detail.plan.digest}
            </Text>
            <Button disabled={busy} onClick={() => void exportText('json')}>
              View JSON export
            </Button>
            <Button disabled={busy} onClick={() => void exportText('csv')}>
              View CSV export
            </Button>
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Save a new review file to a native-selected folder outside your libraries. The
              generated filename is preserved; an existing file is never replaced. Exports contain
              review data, not original photos or execution approval.
            </Text>
            <Button disabled={busy} onClick={() => void saveFile('json')}>
              Save JSON file…
            </Button>
            <Button disabled={busy} onClick={() => void saveFile('csv')}>
              Save CSV file…
            </Button>
            {savedExport && (
              <Text role="status">
                Review file saved: <bdi>{savedExport.path}</bdi> ·{' '}
                {BigInt(savedExport.bytes).toLocaleString()} bytes verified.
              </Text>
            )}
            {exported && (
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Export text — {exported.filename}</span>
                  </Field.Label>
                  <Textarea
                    rows={10}
                    readOnly
                    value={exported.text}
                    onFocus={(event) => event.currentTarget.select()}
                  />
                </label>
              </Field.Root>
            )}
          </chakra.section>
        )}
      </Box>
    </chakra.dialog>
  );
}
