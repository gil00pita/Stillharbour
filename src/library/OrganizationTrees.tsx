import type { LibraryRoot, OrganizationRow } from '../bindings';

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
    <section className="organization-trees" aria-label="Organization path trees">
      <p className="help">
        Selected paths only. Source folders use components supplied by the backend; excluded,
        unknown and unselected files are outside this view. Proposed folders are not created here.
      </p>
      <details>
        <summary>Before · {rows.length} selected paths</summary>
        <ul>
          {ordered(sources).map(([id, files]) => (
            <li key={id}>
              <details>
                <summary title={`Library ID: ${id}`}>
                  {roots.find((root) => root.id === id)?.label ?? `Library ${id}`} · {files.length}{' '}
                  paths
                </summary>
                {files.some(
                  (row) => !row.sourceComponents?.length || row.sourceComponents.length > 65,
                ) && (
                  <p className="help">
                    Source hierarchy unavailable for some retained paths; those paths are shown
                    literally.
                  </p>
                )}
                <PathBranch node={sourceBranch(files)} source />
              </details>
            </li>
          ))}
        </ul>
      </details>
      <details>
        <summary>
          After · {rows.length - unresolved.length} proposed paths · {unresolved.length} without a
          target
        </summary>
        <p className="help">
          Relative to the selected destination. Paths with review issues stay blocked even when a
          proposed name is shown.
        </p>
        <PathBranch node={destination} />
        {unresolved.length > 0 && (
          <>
            <p>No resolved target</p>
            <ul>
              {unresolved.map((row) => (
                <li key={row.assetId}>
                  <bdi>{row.sourcePath}</bdi> · blocked
                </li>
              ))}
            </ul>
          </>
        )}
      </details>
    </section>
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
    <ul>
      {ordered(node.folders).map(([name, child]) => (
        <li key={`folder:${name}`}>
          <details>
            <summary>
              <bdi>{name}</bdi> /
            </summary>
            <PathBranch node={child} source={source} />
          </details>
        </li>
      ))}
      {node.files.map((row) => (
        <li key={`file:${row.assetId}`}>
          <bdi title={source ? row.sourcePath : undefined}>
            {source
              ? row.sourceComponents?.length && row.sourceComponents.length <= 65
                ? row.sourceComponents.at(-1)
                : row.sourcePath
              : row.targetRelativePath?.split('/').at(-1)}
          </bdi>{' '}
          · {BigInt(row.bytes).toLocaleString()} bytes
          {!source && row.issues.length > 0 && ' · blocked: review issues'}
        </li>
      ))}
    </ul>
  );
}
