import { List, Text, chakra } from '@chakra-ui/react';
import type { LibraryRoot, OrganizationRow } from '../../bindings';

type Branch = { folders: Map<string, Branch>; files: OrganizationRow[] };
const branch = (): Branch => ({ folders: new Map(), files: [] });
const ordered = <T,>(values: Map<string, T>) =>
  [...values].sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));

export function OrganizationTrees({
  rows,
  roots = [],
}: {
  rows: OrganizationRow[];
  roots?: Pick<LibraryRoot, 'id' | 'label'>[];
}) {
  const sources = new Map<string, OrganizationRow[]>();
  const destination = branch();
  const unresolved: OrganizationRow[] = [];
  for (const row of rows) {
    const group = sources.get(row.rootId) ?? [];
    group.push(row);
    sources.set(row.rootId, group);
    if (row.targetRelativePath === null) {
      unresolved.push(row);
      continue;
    }
    // Target separators are defined by the DSL. Source display strings stay
    // literal: a backslash can be a filename character on a different platform.
    const components = row.targetRelativePath.split('/');
    let node = destination;
    for (const component of components.slice(0, -1)) {
      const child = node.folders.get(component) ?? branch();
      node.folders.set(component, child);
      node = child;
    }
    node.files.push(row);
  }
  return (
    <chakra.section
      className="organization-trees"
      css={{
        marginBlock: '16px',
        '& details': { marginBlock: '8px' },
        '& summary': { cursor: 'pointer' },
        '& ul': { paddingInlineStart: '20px' },
      }}
      aria-label="Organization path trees"
    >
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Selected paths only. Source folders use components supplied by the backend; excluded,
        unknown and unselected files are outside this view. Proposed folders are not created here.
      </Text>
      <details>
        <summary>Before · {rows.length} selected paths</summary>
        <List.Root unstyled>
          {ordered(sources).map(([id, files]) => (
            <List.Item unstyled key={id}>
              <details>
                <summary title={`Library ID: ${id}`}>
                  {roots.find((root) => root.id === id)?.label ?? `Library ${id}`} · {files.length}{' '}
                  paths
                </summary>
                {files.some(
                  (row) => !row.sourceComponents?.length || row.sourceComponents.length > 65,
                ) && (
                  <Text
                    className="help"
                    css={{
                      fontSize: '13px',
                      color: '{colors.fg.muted}',
                      lineHeight: '20px',
                      margin: '8px 0',
                    }}
                  >
                    Source hierarchy unavailable for some retained paths; those paths are shown
                    literally.
                  </Text>
                )}
                <PathBranch node={sourceBranch(files)} source />
              </details>
            </List.Item>
          ))}
        </List.Root>
      </details>
      <details>
        <summary>
          After · {rows.length - unresolved.length} proposed paths · {unresolved.length} without a
          target
        </summary>
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Relative to the selected destination. Paths with review issues stay blocked even when a
          proposed name is shown.
        </Text>
        <PathBranch node={destination} />
        {unresolved.length > 0 && (
          <>
            <Text>No resolved target</Text>
            <List.Root unstyled>
              {unresolved.map((row) => (
                <List.Item unstyled key={row.assetId}>
                  <bdi>{row.sourcePath}</bdi> · blocked
                </List.Item>
              ))}
            </List.Root>
          </>
        )}
      </details>
    </chakra.section>
  );
}

function sourceBranch(rows: OrganizationRow[]): Branch {
  const root = branch();
  for (const row of rows) {
    const components = row.sourceComponents;
    let node = root;
    if (components && components.length > 0 && components.length <= 65) {
      for (const component of components.slice(0, -1)) {
        const child = node.folders.get(component) ?? branch();
        node.folders.set(component, child);
        node = child;
      }
    }
    node.files.push(row);
  }
  return root;
}

function PathBranch({ node, source = false }: { node: Branch; source?: boolean }) {
  return (
    <List.Root unstyled>
      {ordered(node.folders).map(([name, child]) => (
        <List.Item unstyled key={`folder:${name}`}>
          <details>
            <summary>
              <bdi>{name}</bdi> /
            </summary>
            <PathBranch node={child} source={source} />
          </details>
        </List.Item>
      ))}
      {node.files.map((row) => (
        <List.Item unstyled key={`file:${row.assetId}`}>
          <bdi title={source ? row.sourcePath : undefined}>
            {source
              ? row.sourceComponents?.length && row.sourceComponents.length <= 65
                ? row.sourceComponents.at(-1)
                : row.sourcePath
              : row.targetRelativePath?.split('/').at(-1)}
          </bdi>{' '}
          · {BigInt(row.bytes).toLocaleString()} bytes
          {!source && row.issues.length > 0 && ' · blocked: review issues'}
        </List.Item>
      ))}
    </List.Root>
  );
}
