import { defaultPreset } from '@/components/OrganizationPresets';
import { Box, Flex, Heading, Table, Text, chakra } from '@chakra-ui/react';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type {
  LibraryOrganizationPreview,
  LibraryStatus,
  OrganizationTemplate,
} from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';
import { PageHeader } from '@/components/PageHeader';
import { Button } from '@/components/Button';
import { issueLabels, OrganizationTemplateFields } from '@/components/OrganizationPreview';
import { OrganizationTemplates } from '@/components/OrganizationTemplates';
import { SavedOrganizationPlans } from '@/components/SavedOrganizationPlans';
import { StructureProtection } from './components/StructureProtection';

const pageSize = 240;
function discard(previewId: string | null) {
  if (previewId)
    void library({ kind: 'organization_library_discard', preview_id: previewId }).catch(() => {
      // Read-only TEMP state is also replaced by the next preview and removed on catalog close.
    });
}

export function StructureFormatter({ status }: { status: LibraryStatus }) {
  const title = useRef<HTMLHeadingElement>(null);
  const results = useRef<HTMLTableCaptionElement>(null);
  const generation = useRef(0);
  const activePreview = useRef<string | null>(null);
  const [plansOpen, setPlansOpen] = useState(false);
  const [template, setTemplate] = useState<OrganizationTemplate>({
    pattern: defaultPreset.pattern,
    event: '',
    missingDate: 'undated',
  });
  const [preview, setPreview] = useState<LibraryOrganizationPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [paging, setPaging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const stale = preview !== null && preview.catalogRevision !== status.revision;
  useEffect(() => {
    title.current?.focus({ preventScroll: true });
    return () => {
      generation.current += 1;
      discard(activePreview.current);
      activePreview.current = null;
    };
  }, []);
  const completedPreviewId = preview?.complete ? preview.previewId : null;
  useEffect(() => {
    if (completedPreviewId) results.current?.focus({ preventScroll: true });
  }, [completedPreviewId]);
  function reset() {
    generation.current += 1;
    discard(activePreview.current);
    activePreview.current = null;
    setPreview(null);
    setBusy(false);
    setPaging(false);
    setError(null);
  }
  function update(patch: Partial<OrganizationTemplate>) {
    reset();
    setTemplate((value) => ({ ...value, ...patch }));
  }
  async function build(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!event.currentTarget.reportValidity()) return;
    reset();
    const request = generation.current;
    setBusy(true);
    try {
      let next = await libraryData(
        { kind: 'organization_library_start', template },
        'organization_library_preview',
      );
      if (request !== generation.current) {
        discard(next.previewId);
        return;
      }
      activePreview.current = next.previewId;
      setPreview(next);
      while (!next.complete) {
        next = await libraryData(
          { kind: 'organization_library_advance', preview_id: next.previewId },
          'organization_library_preview',
        );
        if (request !== generation.current) return;
        setPreview(next);
      }
    } catch (cause) {
      if (request === generation.current) {
        setError(errorMessage(cause));
        setPreview(null);
        discard(activePreview.current);
        activePreview.current = null;
      }
    } finally {
      if (request === generation.current) setBusy(false);
    }
  }
  async function page(offset: number) {
    if (!preview || paging || stale) return;
    const request = generation.current;
    setPaging(true);
    setError(null);
    try {
      const next = await libraryData(
        { kind: 'organization_library_page', preview_id: preview.previewId, offset },
        'organization_library_preview',
      );
      if (request === generation.current) {
        setPreview(next);
        results.current?.focus({ preventScroll: true });
      }
    } catch (cause) {
      if (request === generation.current) setError(errorMessage(cause));
    } finally {
      if (request === generation.current) setPaging(false);
    }
  }
  return (
    <Box className="structure-page">
      <PageHeader>
        <Flex as="header" className="toolbar">
          <Heading as="h1" ref={title} tabIndex={-1}>
            Structure Formater
          </Heading>
          <Flex className="header-actions">
            <Button onClick={() => setPlansOpen(true)}>Saved plans</Button>
          </Flex>
        </Flex>
      </PageHeader>
      <Box className="structure-content">
        <Box as="section" aria-label="Library scope" className="structure-source">
          <Box>
            <Heading as="h2">Entire library</Heading>
            <Text color="fg.muted">
              All indexed photos and videos across every library location. Gallery filters, albums
              and photo selections do not limit this scope.
            </Text>
          </Box>
        </Box>
        <Box
          as="section"
          className="organization-preview organization-inline"
          aria-labelledby="structure-rules-title"
        >
          <Box className="organization-heading">
            <Heading as="h2" unstyled id="structure-rules-title">
              Folder &amp; filename rules
            </Heading>
          </Box>
          <Box className="organization-body">
            <chakra.form onSubmit={(event) => void build(event)} aria-busy={busy}>
              <OrganizationTemplateFields
                template={template}
                busy={busy}
                onChange={update}
                submitLabel="Preview restructure"
              />
            </chakra.form>
            <OrganizationTemplates
              template={template}
              onLoad={(value) => {
                reset();
                setTemplate(value);
              }}
            />
            <StructureProtection />
            {busy && (
              <Box>
                <Text role="status">
                  Checking the entire library…{' '}
                  {preview
                    ? `${preview.processed.toLocaleString()} of ${preview.total.toLocaleString()}`
                    : 'Preparing library scope'}
                </Text>
                <Button onClick={reset}>Cancel preview</Button>
              </Box>
            )}
            {error && (
              <Text role="alert" className="error">
                {error}
              </Text>
            )}
            {stale && (
              <Text role="status">
                The catalog has changed. Build a fresh preview before continuing.
              </Text>
            )}
            {preview?.complete && (
              <Box as="section" aria-label="Proposed library structure" aria-busy={paging}>
                <Text role="status">
                  {preview.total.toLocaleString()} library items ·{' '}
                  {preview.proposed.toLocaleString()} proposed · {preview.blocked.toLocaleString()}{' '}
                  blocked
                </Text>
                <Text className="help">
                  One sequence across the entire library, ordered by library ID and native source
                  path. Unavailable items remain in the preview as blocked. Path conflicts are
                  checked across all pages. Native destination rules and companion bundles still
                  require review.
                </Text>
                {preview.total === 0 ? (
                  <Text>
                    No indexed photos or videos yet. Add a library location in Settings, then scan
                    it.
                  </Text>
                ) : (
                  <>
                    <Box className="organization-table">
                      <Table.Root unstyled>
                        <Table.Caption unstyled captionSide="top" ref={results} tabIndex={-1}>
                          Library items {preview.offset + 1}–{preview.offset + preview.rows.length}{' '}
                          of {preview.total}
                        </Table.Caption>
                        <Table.Header unstyled>
                          <Table.Row unstyled>
                            <Table.ColumnHeader unstyled scope="col">
                              Current path
                            </Table.ColumnHeader>
                            <Table.ColumnHeader unstyled scope="col">
                              Proposed path
                            </Table.ColumnHeader>
                            <Table.ColumnHeader unstyled scope="col">
                              Review
                            </Table.ColumnHeader>
                          </Table.Row>
                        </Table.Header>
                        <Table.Body unstyled>
                          {preview.rows.map((row) => (
                            <Table.Row unstyled key={row.assetId}>
                              <Table.Cell unstyled>
                                <bdi>{row.sourcePath}</bdi>
                                <small>
                                  {status.roots.find((root) => root.id === row.rootId)?.label ??
                                    'Unlisted library'}
                                </small>
                              </Table.Cell>
                              <Table.Cell unstyled>
                                <bdi>{row.targetRelativePath ?? 'No valid target'}</bdi>
                              </Table.Cell>
                              <Table.Cell unstyled>
                                {row.issues.length
                                  ? row.issues.map((issue) => (
                                      <Text key={issue}>{issueLabels[issue]}</Text>
                                    ))
                                  : 'Destination checks pending'}
                                {row.usedUndated && (
                                  <Text>Undated fallback; no embedded capture date</Text>
                                )}
                                {row.companionCandidates > 0 && (
                                  <Text>
                                    {row.companionCandidates} indexed companion candidates; bundle
                                    review required
                                  </Text>
                                )}
                              </Table.Cell>
                            </Table.Row>
                          ))}
                        </Table.Body>
                      </Table.Root>
                    </Box>
                    <Flex gap="3" className="structure-pagination" aria-label="Preview pages">
                      <Button
                        disabled={paging || stale || preview.offset === 0}
                        onClick={() => void page(Math.max(0, preview.offset - pageSize))}
                      >
                        Previous page
                      </Button>
                      <Text>
                        Page {Math.floor(preview.offset / pageSize) + 1} of{' '}
                        {Math.ceil(preview.total / pageSize)}
                      </Text>
                      <Button
                        disabled={
                          paging || stale || preview.offset + preview.rows.length >= preview.total
                        }
                        onClick={() => void page(preview.offset + pageSize)}
                      >
                        Next page
                      </Button>
                    </Flex>
                  </>
                )}
              </Box>
            )}
          </Box>
          <Box className="organization-footer">
            <Text className="help" id="structure-execution-help">
              Restructuring originals is not enabled yet. It requires a reviewed whole-library plan,
              destination and companion checks, and proven recovery. This preview does not move or
              rename files.
            </Text>
            <Button disabled aria-describedby="structure-execution-help structure-protection-help">
              Run restructure
            </Button>
          </Box>
        </Box>
      </Box>
      {plansOpen && <SavedOrganizationPlans onClose={() => setPlansOpen(false)} />}
    </Box>
  );
}
