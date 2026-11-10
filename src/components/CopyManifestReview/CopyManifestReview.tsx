import { Box, Heading, List, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  BundleConfirmation,
  CopyManifest,
  CopyManifestIssueKind,
  OrganizationPlanDetail,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { CopyAssessmentReview } from '@/components/CopyAssessmentReview';
import { VerifiedCopyWorkflow } from '@/components/VerifiedCopyWorkflow';
import { ImportComparisonReview } from '@/components/ImportComparisonReview';

const issues: Record<CopyManifestIssueKind, string> = {
  source_has_multiple_targets: 'A source has different proposed destinations across bundles',
  target_has_multiple_sources: 'Different sources have the same proposed destination',
  possible_case_collision: 'Proposed destinations may be equivalent under native case rules',
  file_directory_collision: 'A proposed file is also needed as a folder',
};
const PAGE_SIZE = 64;

export function CopyManifestReview({
  detail,
  choices,
}: {
  detail: OrganizationPlanDetail;
  choices: Record<string, BundleConfirmation>;
}) {
  const [manifest, setManifest] = useState<CopyManifest | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const request = useRef(0);
  const result = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  useEffect(() => {
    if (manifest) result.current?.focus();
  }, [manifest]);
  const decisionIds = detail.plan.preview.rows.map((row) => choices[row.assetId]?.id);
  const complete =
    decisionIds.length > 0 &&
    detail.plan.preview.rows.every(
      (row) =>
        choices[row.assetId]?.planId === detail.plan.id &&
        choices[row.assetId]?.assetId === row.assetId,
    );
  const changed =
    detail.stale ||
    (manifest !== null &&
      (manifest.planId !== detail.plan.id ||
        manifest.draftDigest !== detail.plan.digest ||
        manifest.bundleConfirmationIds.length !== decisionIds.length ||
        manifest.bundleConfirmationIds.some((id) => !decisionIds.includes(id))));
  async function compile() {
    if (pending || !complete || detail.stale) return;
    const token = ++request.current;
    setPending(true);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_manifest',
          input: {
            planId: detail.plan.id,
            bundleConfirmationIds: decisionIds.filter((id): id is string => id !== undefined),
          },
        },
        'organization_copy_manifest',
      );
      if (request.current !== token) return;
      if (value.planId !== detail.plan.id || value.draftDigest !== detail.plan.digest)
        throw new Error('The copy review belongs to a different draft. Reopen the saved plan.');
      setManifest(value);
      setPage(0);
    } catch (error) {
      if (request.current === token) setError(errorMessage(error));
    } finally {
      if (request.current === token) setPending(false);
    }
  }
  const pages = manifest ? Math.max(1, Math.ceil(manifest.files.length / PAGE_SIZE)) : 1;
  return (
    <chakra.section
      className="organization-destination"
      css={{
        margin: '24px 0',
        padding: '16px',
        border: '1px solid {colors.border}',
        borderRadius: 'var(--control-radius)',
        '& h3': { marginTop: '0' },
        '& bdi': { whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' },
      }}
      aria-label="Compiled copy review"
      aria-busy={pending}
    >
      <Heading as="h3" unstyled>
        Review all selected files together
      </Heading>
      <Text>
        Choose a companion decision for each photo. Compilation rechecks those exact inventories and
        combines identical source-to-target mappings. Conflicting mappings stay blocked for review.
      </Text>
      <Text>
        {decisionIds.filter(Boolean).length} of {decisionIds.length} photos have an explicitly
        chosen decision.
      </Text>
      <Button
        type="button"
        disabled={pending || !complete || detail.stale}
        onClick={() => void compile()}
      >
        {pending ? 'Compiling copy review…' : 'Compile copy review'}
      </Button>
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
      {manifest && (
        <>
          <Heading as="h4" unstyled ref={result} tabIndex={-1}>
            Compiled file manifest
          </Heading>
          {changed && (
            <Text role="alert">
              The draft or chosen companion scope changed. Compile a fresh review.
            </Text>
          )}
          <Text>
            {manifest.files.length} proposed file outputs ·{' '}
            {BigInt(manifest.selectedBytes).toLocaleString()} bytes. Identical overlapping mappings
            are counted once.
          </Text>
          <Text>
            Copying requires separate native preparation and byte-verification reviews. This
            retained manifest does not approve copying. Recovery and metadata fidelity limits apply.
          </Text>
          {manifest.issues.length > 0 && (
            <Text role="alert">
              {manifest.issues.length} conflicts block the proposed copy. Build a new draft or
              choose a compatible companion scope.
            </Text>
          )}
          {manifest.issues.length > 0 && (
            <List.Root unstyled>
              {manifest.issues.slice(0, 64).map((issue, index) => (
                <List.Item unstyled key={index}>
                  {issues[issue.kind]} ({issue.fileIds.length} files)
                </List.Item>
              ))}
            </List.Root>
          )}
          {manifest.issues.length > 64 && (
            <Text>
              {manifest.issues.length - 64} additional conflicts are retained in the manifest.
            </Text>
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
              '& caption': { textAlign: 'left', paddingBottom: '12px', color: '{colors.fg.muted}' },
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
            <Table.Root unstyled>
              <Table.Caption unstyled captionSide="top">
                Compiled files, page {page + 1} of {pages}
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled>Source filename</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Proposed destination</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Bytes</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Photo bundles</Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {manifest.files.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE).map((file) => (
                  <Table.Row unstyled key={file.id}>
                    <Table.Cell unstyled>
                      <bdi title={`Library ${file.rootId}`}>{file.sourceName}</bdi>
                    </Table.Cell>
                    <Table.Cell unstyled>
                      <bdi>{file.targetRelativePath}</bdi>
                    </Table.Cell>
                    <Table.Cell unstyled>{BigInt(file.bytes).toLocaleString()}</Table.Cell>
                    <Table.Cell unstyled>{file.bundleAssetIds.length}</Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Button type="button" disabled={page === 0} onClick={() => setPage((value) => value - 1)}>
            Previous compiled files
          </Button>
          <Button
            type="button"
            disabled={page + 1 >= pages}
            onClick={() => setPage((value) => value + 1)}
          >
            Next compiled files
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
            Manifest fingerprint: {manifest.digest}
          </Text>
          {!changed && <CopyAssessmentReview key={manifest.digest} manifest={manifest} />}
          {!changed && manifest.issues.length === 0 && (
            <ImportComparisonReview key={`compare-${manifest.digest}`} manifest={manifest} />
          )}
          {!changed && manifest.issues.length === 0 && (
            <VerifiedCopyWorkflow key={`copy-${manifest.digest}`} manifest={manifest} />
          )}
        </>
      )}
    </chakra.section>
  );
}
