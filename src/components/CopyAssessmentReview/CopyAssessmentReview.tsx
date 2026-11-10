import { Box, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type {
  CopyAssessment,
  CopyFidelityAspect,
  CopyFidelityStatus,
  CopyManifest,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';

const aspects: Record<CopyFidelityAspect, string> = {
  file_bytes: 'File bytes',
  confirmed_companions: 'Confirmed companions',
  timestamps: 'Timestamps',
  permissions_and_ownership: 'Permissions and ownership',
  extended_attributes_and_resource_forks: 'Extended attributes and resource forks',
  alternate_data_streams: 'Alternate data streams',
  sparse_and_compression_layout: 'Sparse and compression layout',
};
const statuses: Record<CopyFidelityStatus, string> = {
  verification_required: 'Full byte verification required',
  separate_files_selected: 'Selected as separate files',
  not_preserved_or_verified: 'Not preserved or verified',
};
export function CopyAssessmentReview({ manifest }: { manifest: CopyManifest }) {
  const [report, setReport] = useState<CopyAssessment | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  useEffect(() => {
    if (report) heading.current?.focus();
  }, [report]);
  async function check() {
    if (pending) return;
    const token = ++request.current;
    setPending(true);
    setReport(null);
    setError(null);
    try {
      const value = await libraryData(
        {
          kind: 'organization_copy_assessment',
          plan_id: manifest.planId,
          manifest_id: manifest.id,
        },
        'organization_copy_assessment',
      );
      if (request.current !== token) return;
      if (
        value.planId !== manifest.planId ||
        value.manifestId !== manifest.id ||
        value.manifestDigest !== manifest.digest
      ) {
        throw new Error(
          'The space report belongs to a different review. Compile a fresh manifest.',
        );
      }
      setReport(value);
    } catch (error) {
      if (request.current === token) setError(errorMessage(error));
    } finally {
      if (request.current === token) setPending(false);
    }
  }
  const capacity = report?.capacity;
  return (
    <chakra.section aria-label="Copy space and fidelity" aria-busy={pending}>
      <Button type="button" disabled={pending} onClick={() => void check()}>
        {pending ? 'Checking copy space…' : 'Check copy space and fidelity'}
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
      {report && capacity && (
        <>
          <Heading as="h4" unstyled ref={heading} tabIndex={-1}>
            Copy space estimate and fidelity
          </Heading>
          <Text>
            {capacity.status === 'insufficient_for_estimate'
              ? 'Observed space is below the copy estimate. Choose another destination or reduce the selection.'
              : 'Observed space meets this planning estimate.'}{' '}
            Space is not reserved; recheck before approval and copying.
          </Text>
          {capacity.status === 'insufficient_for_estimate' && (
            <Text role="alert">Insufficient observed space blocks copying.</Text>
          )}
          <dl>
            {(
              [
                ['Selected file bytes', capacity.logicalBytes],
                ['Data rounded per file', capacity.roundedDataBytes],
                ['Metadata allowance', capacity.metadataAllowanceBytes],
                ['Journal allowance', capacity.journalAllowanceBytes],
                ['Headroom', capacity.headroomBytes],
                ['Estimated peak additional space', capacity.estimatedPeakAdditionalBytes],
                ['Observed available space', capacity.observedAvailableBytes],
                ['Observed total volume space', capacity.observedTotalBytes],
                ['Native allocation unit', capacity.allocationUnitBytes],
              ] satisfies [string, string][]
            ).map(([label, bytes]) => (
              <Box key={label}>
                <dt>{label}</dt>
                <dd>{BigInt(bytes).toLocaleString()} bytes</dd>
              </Box>
            ))}
          </dl>
          <Text>
            {capacity.outputDirectories} output folders, including the pending output root. Staging
            and publication use the same file objects. Allowances are estimates, not a filesystem
            capacity guarantee.
          </Text>
          <Text>
            Originals stay in place. This report does not verify any copied content or independent
            backup. The current copy engine has these fidelity limits:
          </Text>
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
                Copy fidelity scope
              </Table.Caption>
              <Table.Header unstyled>
                <Table.Row unstyled>
                  <Table.ColumnHeader unstyled>Aspect</Table.ColumnHeader>
                  <Table.ColumnHeader unstyled>Current support</Table.ColumnHeader>
                </Table.Row>
              </Table.Header>
              <Table.Body unstyled>
                {report.fidelity.map((item) => (
                  <Table.Row unstyled key={item.aspect}>
                    <Table.Cell unstyled>{aspects[item.aspect]}</Table.Cell>
                    <Table.Cell unstyled>{statuses[item.status]}</Table.Cell>
                  </Table.Row>
                ))}
              </Table.Body>
            </Table.Root>
          </Box>
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Observed {report.observedAt}. This report does not approve copying.
          </Text>
        </>
      )}
    </chakra.section>
  );
}
