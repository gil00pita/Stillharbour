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
          '& .organization-heading': { paddingInline: '0' },
          '& .organization-body': { paddingInline: '0', overflow: 'visible' },
          '& .organization-footer': { paddingInline: '0' },
          '& .organization-heading h2': { fontSize: '18px' },
        },
      }}
      ref={inline ? undefined : dialog}
      aria-labelledby="organization-title"
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
        <Box>
          <Heading as="h2" unstyled id="organization-title">
            Folder &amp; filename template
          </Heading>
          {!inline && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
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
      >
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
        {preview && (
          <chakra.section aria-label="Proposed organization">
            <Text role="status">
              {preview.proposed} proposed · {preview.blocked} blocked ·{' '}
              {BigInt(preview.selectedBytes).toLocaleString()} selected file bytes
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
              Relative paths only. Destination conflicts, native case and Unicode rules, companions,
              metadata fidelity and free space still need review. Sequence numbers follow library ID
              and native source path order.
            </Text>
            {template.missingDate === 'skip' ? (
              <Text
                className="help"
                css={{
                  fontSize: '13px',
                  color: '{colors.fg.muted}',
                  lineHeight: '20px',
                  margin: '8px 0',
                }}
              >
                Skipping missing dates is supported in Structure Formater’s whole-library preview.
                Destination and copy planning are unavailable for this saved rule.
              </Text>
            ) : (
              <OrganizationDestination
                catalogId={catalogId}
                preferredImportRoot={preferredImportRoot}
                key={preview.fingerprint}
                preview={preview}
                revision={revision}
                templateReference={templateReference}
                roots={roots}
              />
            )}
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
                        {row.skipReason
                          ? 'Skipped · missing capture date; selected-photo plans do not support skipping'
                          : row.issues.length
                            ? row.issues.map((issue) => (
                                <Text key={issue}>{issueLabels[issue]}</Text>
                              ))
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
      <chakra.footer
        className="organization-footer"
        css={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '24px',
          padding: '20px 24px',
          flexShrink: '0',
          '& p': { margin: '0' },
          borderTop: '1px solid {colors.border}',
          '& button': { flexShrink: '0' },
          '@media (max-width: 760px)': {
            alignItems: 'flex-start',
            flexDirection: 'column',
            gap: '12px',
          },
        }}
      >
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
          id="organization-execution-help"
        >
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
