import { expect, it } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { IntegrityFindings } from './IntegrityFindings';
import { member } from '../DuplicateStudio/fixtures';
it('separates a changed baseline and offline presence from untested content without claiming damage', () => {
  render(
    <IntegrityFindings
      rows={[
        member('changed', { checksum: 'changed_since_baseline' }),
        member('offline', { inventory: 'offline' }),
        member('unchecked'),
      ]}
    />,
  );
  fireEvent.click(screen.getByRole('button', { name: 'Needs attention' }));
  expect(screen.getByRole('rowheader', { name: /changed.png/ })).toBeVisible();
  expect(screen.getByRole('rowheader', { name: /offline.png/ })).toBeVisible();
  expect(screen.queryByRole('rowheader', { name: /unchecked.png/ })).not.toBeInTheDocument();
  expect(screen.getByText(/it does not prove damage/)).toBeVisible();
});
