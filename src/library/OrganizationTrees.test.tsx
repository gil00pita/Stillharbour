import { render, screen, within } from '@testing-library/react';
import { expect, it } from 'vitest';
import type { OrganizationRow } from '../bindings';
import { OrganizationTrees } from './OrganizationTrees';

function row(
  id: string,
  root: string,
  target: string | null,
  issues: OrganizationRow['issues'] = [],
): OrganizationRow {
  return {
    assetId: id,
    rootId: root,
    sourcePath: 'literal\\<photo>.JPG',
    sourceRevision: '1',
    targetRelativePath: target,
    sequence: 1,
    bytes: '9007199254740993',
    issues,
    usedUndated: false,
    companionCandidates: 0,
  };
}
it('retains colliding files, a file/folder collision and unresolved targets without interpreting source names', () => {
  render(
    <OrganizationTrees
      rows={[
        row('a', 'r1', '__proto__/photo.JPG', ['collision']),
        row('b', 'r2', '__proto__/photo.JPG', ['collision']),
        row('c', 'r1', '__proto__', ['collision']),
        row('d', 'r1', null, ['source_unavailable']),
      ]}
      roots={[{ id: 'r1', label: 'Family <root>' }]}
    />,
  );
  expect(screen.getByText('Before · 4 selected paths')).toBeInTheDocument();
  expect(screen.getByText('After · 3 proposed paths · 1 without a target')).toBeInTheDocument();
  expect(screen.getByTitle('Library ID: r1')).toHaveTextContent('Family <root> · 3 paths');
  expect(screen.getByTitle('Library ID: r2')).toHaveTextContent('Library r2 · 1 paths');
  const before = screen.getByText('Before · 4 selected paths').parentElement!;
  expect(within(before).getAllByText('literal\\<photo>.JPG')).toHaveLength(4);
  const after = screen.getByText('After · 3 proposed paths · 1 without a target').parentElement!;
  expect(within(after).getAllByText('photo.JPG')).toHaveLength(2);
  expect(within(after).getAllByText('__proto__')).toHaveLength(2);
  expect(within(after).getByText('No resolved target')).toBeInTheDocument();
  expect(after.textContent).toContain(BigInt('9007199254740993').toLocaleString());
  expect(document.querySelector('photo')).toBeNull();
});
it('renders only frozen selection rows without loading the filesystem or expanding a catalog scope', () => {
  const { rerender } = render(<OrganizationTrees rows={[row('a', 'root', '2026/photo.JPG')]} />);
  expect(screen.getByText(/unknown and unselected/)).toBeInTheDocument();
  expect(document.querySelectorAll('details[open]')).toHaveLength(0);
  rerender(<OrganizationTrees rows={[row('b', 'root', '2027/new.JPG')]} />);
  expect(screen.queryByText('2026')).not.toBeInTheDocument();
  expect(screen.queryByText('photo.JPG')).not.toBeInTheDocument();
  expect(screen.getByText('2027')).toBeInTheDocument();
});

it('uses backend native components without parsing source display strings or filename backslashes', () => {
  const a = {
    ...row('a', 'root', 'new/a.JPG'),
    sourcePath: 'display must stay literal',
    sourceComponents: ['Family', 'Trips', 'a\\b.JPG'],
  };
  const b = {
    ...row('b', 'root', 'new/b.JPG'),
    sourceComponents: ['Family', '__proto__', '<photo>.JPG'],
  };
  render(<OrganizationTrees rows={[a, b]} />);
  const before = screen.getByText('Before · 2 selected paths').parentElement!;
  expect(within(before).getByText('Family')).toBeInTheDocument();
  expect(within(before).getByText('Trips')).toBeInTheDocument();
  expect(within(before).getByText('__proto__')).toBeInTheDocument();
  expect(within(before).getByText('a\\b.JPG')).toHaveAttribute(
    'title',
    'display must stay literal',
  );
  expect(within(before).getByText('<photo>.JPG')).toBeInTheDocument();
  expect(document.querySelector('photo')).toBeNull();
});
