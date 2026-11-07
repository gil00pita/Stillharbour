import { Box, Table, Text } from '@chakra-ui/react';
import type { Ref } from 'react';
import { issueLabels } from '@/components/OrganizationPreview';
import type { LibraryOrganizationPreview, LibraryStatus, OrganizationRow } from '../../../bindings';
function RowStatus({ row }: { row: OrganizationRow }) {
  if (row.skipReason) return <Text>Skipped · no indexed capture date</Text>;
  if (row.issues.length)
    return (
      <>
        {row.issues.map((issue) => (
          <Text key={issue}>{issueLabels[issue]}</Text>
        ))}
      </>
    );
  if (row.usedUndated) return <Text>Undated folder</Text>;
  if (row.sourceComponents?.join('/') === row.targetRelativePath) return <Text>Unchanged</Text>;
  return <Text>Proposed</Text>;
}
export function StructureReviewTable({
  preview,
  roots,
  busy,
  caption,
  available,
}: {
  preview: LibraryOrganizationPreview;
  roots: LibraryStatus['roots'];
  busy: boolean;
  caption: Ref<HTMLTableCaptionElement>;
  available: boolean;
}) {
  if (!available)
    return (
      <Box className="structure-review-table">
        <Text>
          Results are unavailable because the latest query failed. Try another filter or return to
          editing to rebuild the preview.
        </Text>
      </Box>
    );
  return (
    <Box className="structure-review-table" aria-busy={busy}>
      <Table.Root unstyled>
        <Table.Caption unstyled ref={caption} tabIndex={-1} captionSide="top">
          {busy
            ? 'Updating preview results…'
            : preview.matching === 0
              ? 'No matching photos'
              : `Library items ${preview.offset + 1}–${preview.offset + preview.rows.length} of ${preview.matching.toLocaleString()} matching`}
        </Table.Caption>
        <Table.Header unstyled>
          <Table.Row unstyled>
            {['Photo', 'Current path', 'Proposed path', 'Review'].map((title) => (
              <Table.ColumnHeader unstyled scope="col" key={title}>
                {title}
              </Table.ColumnHeader>
            ))}
          </Table.Row>
        </Table.Header>
        <Table.Body unstyled>
          {preview.rows.map((row) => (
            <Table.Row unstyled key={row.assetId}>
              <Table.Cell unstyled>
                <bdi>{row.sourceComponents?.at(-1) ?? row.sourcePath}</bdi>
                <small>
                  {roots.find((root) => root.id === row.rootId)?.label ?? 'Unlisted library'}
                </small>
              </Table.Cell>
              <Table.Cell unstyled>
                <bdi>{row.sourcePath}</bdi>
              </Table.Cell>
              <Table.Cell unstyled>
                <bdi>
                  {row.skipReason ? row.sourcePath : (row.targetRelativePath ?? 'No valid target')}
                </bdi>
                {row.skipReason && <small>Kept in place</small>}
              </Table.Cell>
              <Table.Cell unstyled>
                <RowStatus row={row} />
                {row.captureMissing && !row.usedUndated && !row.skipReason && (
                  <small>No indexed capture date</small>
                )}
                {row.companionCandidates > 0 && (
                  <small>
                    {row.companionCandidates} companion candidates; bundle review required
                  </small>
                )}
              </Table.Cell>
            </Table.Row>
          ))}
        </Table.Body>
      </Table.Root>
    </Box>
  );
}
