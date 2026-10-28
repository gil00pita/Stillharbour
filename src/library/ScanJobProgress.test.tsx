import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { ScanJob } from '../bindings';
import { ScanJobProgress } from './ScanJobProgress';

const job: ScanJob = {
  id: 'generated-scan',
  rootId: 'generated-root',
  state: 'running',
  discovered: 20,
  indexed: 20,
  skipped: 0,
  errors: 0,
  message: 'Indexing files; originals unchanged',
  startedAt: '2026-10-27',
};

describe('scan job progress', () => {
  it('does not claim a percentage while discovery can still find more files', () => {
    const { rerender } = render(<ScanJobProgress job={job} label="Generated library" />);
    const progress = screen.getByRole('progressbar', {
      name: 'Scan progress for Generated library',
    });
    expect(progress).not.toHaveAttribute('aria-valuenow');
    expect(progress).toHaveAccessibleDescription('Scanning · total files not yet known');
    rerender(<ScanJobProgress job={{ ...job, discovered: 50 }} label="Generated library" />);
    expect(progress).not.toHaveAttribute('aria-valuenow');
  });
  it('keeps cancellation pending until the backend confirms it stopped', () => {
    const { rerender } = render(
      <ScanJobProgress job={{ ...job, state: 'cancel_requested' }} label="Library" />,
    );
    expect(screen.getByRole('progressbar')).toHaveAccessibleDescription('Stopping scan…');
    rerender(<ScanJobProgress job={{ ...job, state: 'canceled' }} label="Library" />);
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(screen.getByText('Scan canceled')).toBeVisible();
  });
  it('shows completion only when the backend reports completed, even for an empty scan', () => {
    render(
      <ScanJobProgress
        job={{ ...job, state: 'completed', discovered: 0, indexed: 0 }}
        label="Library"
      />,
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100');
    expect(screen.getByRole('progressbar')).toHaveAccessibleDescription('Scan complete');
  });
  it.each([
    ['partial', 'Scan finished with issues'],
    ['interrupted', 'Scan interrupted'],
    ['failed', 'Scan failed'],
  ])('never presents %s as ongoing or successfully completed', (state, description) => {
    render(<ScanJobProgress job={{ ...job, state }} label="Library" />);
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    expect(screen.getByText(description)).toBeVisible();
  });
});
