import { expect, it, vi } from 'vitest';
import { render, screen } from '../../test/render';
import { BackupScreen } from '@/components/BackupScreen';
import type { LibraryStatus } from '../../bindings';
vi.mock('@/components/SnapshotWorkflow', () => ({
  SnapshotWorkflow: () => <section aria-label="Versioned backup workflow" />,
}));
it('opens versioned backup directly, without comparison or a mode switch', () => {
  render(<BackupScreen status={{ roots: [], jobs: [] } as unknown as LibraryStatus} />);
  expect(screen.getByRole('heading', { level: 1, name: 'Backup' })).toBeVisible();
  expect(screen.getByRole('region', { name: 'Versioned backup workflow' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Safe copy' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Compare' })).not.toBeInTheDocument();
});
