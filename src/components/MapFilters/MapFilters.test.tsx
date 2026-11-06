import { useState } from 'react';
import { beforeEach, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen, waitFor } from '../../test/render';
import { defaultQuery } from '../../library/model';
import { MapFilters } from '@/components/MapFilters';

beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

function Harness() {
  const [query, setQuery] = useState(defaultQuery);
  return (
    <MapFilters
      query={query}
      roots={[]}
      unlocated={false}
      unavailable={2}
      onFilter={(patch) => setQuery((current) => ({ ...current, ...patch }))}
      onLocation={() => {}}
    />
  );
}
it('starts hidden, retains active values and dismisses with Escape back to its trigger', async () => {
  render(<Harness />);
  const trigger = screen.getByRole('button', { name: 'Map filters' });
  expect(trigger).toHaveAttribute('aria-expanded', 'false');
  expect(screen.queryByLabelText('From')).not.toBeInTheDocument();
  act(() => trigger.focus());
  fireEvent.click(trigger);
  expect(await screen.findByRole('dialog', { name: 'Map filters' })).toBeVisible();
  fireEvent.change(screen.getByLabelText('From'), { target: { value: '2026-01-01' } });
  fireEvent.click(screen.getByRole('button', { name: 'Close filters' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  const active = screen.getByRole('button', { name: 'Map filters (1 active)' });
  await waitFor(() => expect(active).toHaveFocus());
  fireEvent.click(active);
  const date = await screen.findByLabelText('From');
  expect(date).toHaveValue('2026-01-01');
  // Portalled focus and dismiss listeners are installed after mounting.
  // Exercise Escape only after the reopened popover has received focus.
  await waitFor(() =>
    expect(
      screen.getByRole('dialog', { name: 'Map filters' }).contains(document.activeElement),
    ).toBe(true),
  );
  act(() => date.focus());
  fireEvent.keyDown(date, { key: 'Escape', code: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  await waitFor(() => expect(active).toHaveFocus());
});
