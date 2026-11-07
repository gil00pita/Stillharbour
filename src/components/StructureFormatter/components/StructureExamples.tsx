import { Box, Heading, List, Text, chakra } from '@chakra-ui/react';
import { useEffect, useState } from 'react';
import { SegmentedControl } from '@/components/SegmentedControl';
import { issueLabels } from '@/components/OrganizationPreview';
import type { OrganizationExamplePreview, OrganizationTemplate } from '../../../bindings';
import { errorMessage, libraryData } from '../../../ipc';

type Node = { children: Map<string, Node>; file: boolean };
function tree(paths: string[]) {
  const root: Node = { children: new Map(), file: false };
  for (const path of paths) {
    const parts = path.split('/');
    let node = root;
    for (const [index, part] of parts.entries()) {
      if (!node.children.has(part))
        node.children.set(part, { children: new Map(), file: index === parts.length - 1 });
      node = node.children.get(part)!;
    }
  }
  return root;
}
function Tree({ node }: { node: Node }) {
  return (
    <List.Root unstyled className="structure-sample-tree">
      {[...node.children].map(([name, child]) => (
        <List.Item unstyled key={name}>
          {child.file ? (
            <Text as="span">
              <bdi>{name}</bdi>
            </Text>
          ) : (
            <chakra.details open>
              <chakra.summary>
                <bdi>{name}</bdi>/
              </chakra.summary>
              <Tree node={child} />
            </chakra.details>
          )}
        </List.Item>
      ))}
    </List.Root>
  );
}
export function StructureExamples({ template }: { template: OrganizationTemplate }) {
  const [view, setView] = useState<'tree' | 'paths'>('tree');
  const [result, setResult] = useState<{
    key: string;
    value: OrganizationExamplePreview | null;
    error: string | null;
  } | null>(null);
  const key = JSON.stringify(template);
  const latest = result?.key === key ? result : null;
  useEffect(() => {
    let closed = false;
    const timeout = window.setTimeout(() => {
      void libraryData(
        {
          kind: 'organization_template_examples',
          template: JSON.parse(key) as OrganizationTemplate,
        },
        'organization_template_examples',
      ).then(
        (value) => {
          if (!closed) setResult({ key, value, error: null });
        },
        (cause: unknown) => {
          if (!closed) setResult({ key, value: null, error: errorMessage(cause) });
        },
      );
    }, 180);
    return () => {
      closed = true;
      window.clearTimeout(timeout);
    };
  }, [key]);
  const rows = latest?.value?.rows ?? [];
  return (
    <Box
      as="aside"
      className="structure-examples"
      aria-label="Example structure"
      aria-busy={!latest}
    >
      <Heading as="h2" fontSize="xl" mb="2">
        A place for every photo
      </Heading>
      <Text color="fg.muted" mb="5">
        Illustrative examples update with your rules. Your library is checked on the next screen.
      </Text>
      <SegmentedControl
        label="Example view"
        value={view}
        disabled={false}
        choices={[
          { value: 'tree', label: 'Folder tree' },
          { value: 'paths', label: 'Sample paths' },
        ]}
        onChange={setView}
      />
      {!latest ? (
        <Text mt="5">Updating examples…</Text>
      ) : latest.error ? (
        <Text role="alert" mt="5">
          {latest.error}
        </Text>
      ) : (
        <>
          {view === 'tree' ? (
            <Box className="structure-tree-frame">
              <Text fontWeight="semibold" mb="3">
                Example library
              </Text>
              <Tree
                node={tree(
                  rows.flatMap((row) => (row.targetRelativePath ? [row.targetRelativePath] : [])),
                )}
              />
            </Box>
          ) : (
            <List.Root unstyled className="structure-sample-paths">
              {rows.map((row) => (
                <List.Item unstyled key={row.assetId}>
                  <Text color="fg.muted">
                    <bdi>{row.sourcePath}</bdi>
                  </Text>
                  <Text>
                    <bdi>
                      {row.targetRelativePath ??
                        (row.skipReason ? 'Skipped — kept in place' : 'No valid target')}
                    </bdi>
                  </Text>
                </List.Item>
              ))}
            </List.Root>
          )}
          <Text
            className="help"
            css={{
              fontSize: '13px',
              color: '{colors.fg.muted}',
              lineHeight: '20px',
              margin: '8px 0',
            }}
          >
            Four synthetic examples, including one without a capture date. No library files are
            inspected for these examples.
          </Text>
          {rows.some((row) => row.skipReason) && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              The example without a required date is skipped.
            </Text>
          )}
          {rows.some((row) => row.usedUndated) && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              The example without a capture date uses Undated and its original filename.
            </Text>
          )}
          {[...new Set(rows.flatMap((row) => row.issues))].map((issue) => (
            <Text
              key={issue}
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              {issueLabels[issue]}
            </Text>
          ))}
        </>
      )}
    </Box>
  );
}
