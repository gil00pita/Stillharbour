import { expect, it } from 'vitest';
import { fireEvent, render, screen } from '../../test/render';
import { PageHeader } from '@/components/PageHeader';
it('suppresses native context menus on titles while keeping input context menus available', () => {
  render(
    <PageHeader>
      <header className="toolbar">
        <h1>Photos</h1>
        <input aria-label="Search" />
      </header>
    </PageHeader>,
  );
  expect(screen.getByRole('banner')).toHaveClass('toolbar', 'app-page-header');
  expect(fireEvent.contextMenu(screen.getByRole('heading'))).toBe(false);
  expect(fireEvent.contextMenu(screen.getByRole('textbox'))).toBe(true);
});
