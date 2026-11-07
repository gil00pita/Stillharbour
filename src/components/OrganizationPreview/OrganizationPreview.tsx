import { issueLabels } from './issue-labels';
import { defaultPreset } from '@/components/OrganizationPresets';
import { OrganizationTemplateFields } from './components/OrganizationTemplateFields';
import { Field, chakra, Box, Heading, Table, Text } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import type {
  OrganizationPreview as Preview,
  OrganizationTemplate,
  TemplateRef,
  ImportDestinationPreference,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { OrganizationCompanions } from '@/components/OrganizationCompanions';
import {
  OrganizationDestination,
  type DestinationRootOption,
} from '@/components/OrganizationDestination';
import { OrganizationTemplates } from '@/components/OrganizationTemplates';
import { OrganizationTrees } from '@/components/OrganizationTrees';

export function OrganizationPreview({
  inline = false,
  catalogId = null,
  preferredImportRoot = null,
  assetIds,
  revision,
  roots,
  onClose,
}: {
  inline?: boolean;
  catalogId?: string | null;
  preferredImportRoot?: ImportDestinationPreference | null;
  assetIds: string[];
  revision: string;
  roots: DestinationRootOption[];
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const resultsCaption = useRef<HTMLTableCaptionElement>(null);
  const requestId = useRef(0);
  const [template, setTemplate] = useState<OrganizationTemplate>({
    pattern: defaultPreset.pattern,
    event: '',
    missingDate: 'undated',
  });
  const [preview, setPreview] = useState<Preview | null>(null);
  const [templateReference, setTemplateReference] = useState<TemplateRef | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  useEffect(() => {
    if (preview) {
      resultsCaption.current?.focus({ preventScroll: true });
      resultsCaption.current?.scrollIntoView({ block: 'start' });
    }
  }, [preview]);
  useEffect(() => {
    const element = inline ? null : dialog.current;
    const opener = document.activeElement;
    if (element && !element.open) element.showModal();
    return () => {
      requestId.current += 1;
      element?.close();
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, [inline]);
  function update(patch: Partial<OrganizationTemplate>) {
    setTemplateReference(null);
    setTemplate((value) => {
      const next = { ...value, ...patch };
      if (next.filenameStemCase === 'preserve') delete next.filenameStemCase;
      return next;
    });
    setPreview(null);
    setError(null);
  }
  async function build(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!event.currentTarget.reportValidity()) return;
    const id = ++requestId.current;
    setBusy(true);
    setError(null);
    setPreview(null);
    try {
      const result = await libraryData(
        { kind: 'organization_preview', asset_ids: assetIds, template },
        'organization_preview',
      );
      if (requestId.current === id) setPreview(result);
    } catch (error) {
      if (requestId.current === id) setError(errorMessage(error));
    } finally {
      if (requestId.current === id) setBusy(false);
    }
  }
  const rows = preview?.rows.filter((row) =>
    `${row.sourcePath} ${row.targetRelativePath ?? ''}`
      .toLocaleLowerCase()
      .includes(search.toLocaleLowerCase()),
  );
  const Surface = inline ? chakra.section : chakra.dialog;
  return (
    <Surface
      className={`organization-preview ${inline ? 'organization-inline' : ''}`}
      ref={inline ? undefined : dialog}
      aria-labelledby="organization-title"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
    >
      <chakra.header className="organization-heading">
        <Box>
          <Heading as="h2" unstyled id="organization-title">
            Folder &amp; filename template
          </Heading>
          {!inline && (
            <Text className="help">
              {assetIds.length} explicitly selected {assetIds.length === 1 ? 'photo' : 'photos'} ·
              Originals stay in place
            </Text>
          )}
        </Box>
        {!inline && (
          <Button onClick={onClose} aria-label="Close organization preview">
            Close
          </Button>
        )}
      </chakra.header>
      <Box className="organization-body">
        {!inline && (
          <Text>
            Preview folder and filename changes using indexed metadata. Originals stay in place.
          </Text>
        )}
        <chakra.form onSubmit={(event) => void build(event)} aria-busy={busy}>
          <OrganizationTemplateFields busy={busy} template={template} onChange={update} />
        </chakra.form>
        <OrganizationTemplates
          template={template}
          onLoad={(value, reference) => {
            requestId.current += 1;
            setBusy(false);
            setTemplate(value);
            setTemplateReference(reference);
            setPreview(null);
            setError(null);
          }}
        />
        {error && (
          <Text role="alert" className="error">
            {error}
          </Text>
        )}
        {preview && (
          <chakra.section aria-label="Proposed organization">
            <Text role="status">
              {preview.proposed} proposed · {preview.blocked} blocked ·{' '}
              {BigInt(preview.selectedBytes).toLocaleString()} selected file bytes
            </Text>
            <Text className="help">
              Relative paths only. Destination conflicts, native case and Unicode rules, companions,
              metadata fidelity and free space still need review. Sequence numbers follow library ID
              and native source path order.
            </Text>
            <OrganizationDestination
              catalogId={catalogId}
              preferredImportRoot={preferredImportRoot}
              key={preview.fingerprint}
              preview={preview}
              revision={revision}
              templateReference={templateReference}
              roots={roots}
            />
            <Field.Root asChild unstyled>
              <label>
                <Field.Label asChild unstyled>
                  <span>Find a path</span>
                </Field.Label>
                <Input
                  type="search"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
              </label>
            </Field.Root>
            <OrganizationTrees rows={preview.rows} roots={roots} />
            <Box className="organization-table">
              <Table.Root unstyled>
                <Table.Caption unstyled captionSide="top" ref={resultsCaption} tabIndex={-1}>
                  Selected photos: current path, proposed relative path and review issues
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
                  {rows?.map((row) => (
                    <Table.Row unstyled key={row.assetId}>
                      <Table.Cell unstyled>
                        <bdi>{row.sourcePath}</bdi>
                        <small title={`Library ID: ${row.rootId}`}>
                          {roots.find((root) => root.id === row.rootId)?.label ??
                            'Unlisted library'}
                        </small>
                      </Table.Cell>
                      <Table.Cell unstyled>
                        <bdi>{row.targetRelativePath ?? 'No valid target'}</bdi>
                      </Table.Cell>
                      <Table.Cell unstyled>
                        {row.issues.length
                          ? row.issues.map((issue) => <Text key={issue}>{issueLabels[issue]}</Text>)
                          : 'Destination checks pending'}
                        {row.usedUndated && <Text>Undated fallback; no embedded capture date</Text>}
                        {row.companionCandidates > 0 && (
                          <Text>
                            {row.companionCandidates} indexed companion candidates; bundle review
                            required
                          </Text>
                        )}
                        <OrganizationCompanions
                          key={`${preview.fingerprint}:${row.assetId}`}
                          assetId={row.assetId}
                          previewRevision={preview.catalogRevision}
                          revision={revision}
                        />
                      </Table.Cell>
                    </Table.Row>
                  ))}
                </Table.Body>
              </Table.Root>
              {rows?.length === 0 && <Text>No paths match your search.</Text>}
            </Box>
          </chakra.section>
        )}
      </Box>
      <chakra.footer className="organization-footer">
        <Text className="help" id="organization-execution-help">
          Copying and in-place changes are unavailable until destination planning, verification and
          recovery checks pass.
        </Text>
        <Button disabled aria-describedby="organization-execution-help">
          Create organized copy
        </Button>
      </chakra.footer>
    </Surface>
  );
}
