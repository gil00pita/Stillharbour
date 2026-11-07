import { expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within, waitFor } from '../../test/render';
import { OrganizationPresets, defaultPreset, presets } from '@/components/OrganizationPresets';

it('shows date-based radio cards with decorative SVG examples and supports custom rules', async () => {
  const onChange = vi.fn();
  const view = render(
    <OrganizationPresets pattern={defaultPreset.pattern} disabled={false} onChange={onChange} />,
  );
  const group = screen.getByRole('radiogroup', { name: 'Folder layout' });
  expect(within(group).getAllByRole('radio')).toHaveLength(8);
  expect(within(group).getByRole('radio', { name: 'By day' })).toBeChecked();
  expect(group.querySelectorAll('svg[aria-hidden="true"]')).toHaveLength(8);
  expect(
    presets
      .filter((preset) => preset.group === 'date')
      .every((preset) => !preset.pattern.includes('{event}')),
  ).toBe(true);
  fireEvent.click(within(group).getByRole('radio', { name: 'Date & time' }));
  await waitFor(() =>
    expect(onChange).toHaveBeenLastCalledWith(
      presets.find((preset) => preset.name === 'Date & time')?.pattern,
    ),
  );
  view.rerender(
    <OrganizationPresets pattern="{original_stem}.{ext}" disabled={false} onChange={onChange} />,
  );
  expect(screen.getByRole('combobox', { name: 'Folder levels' })).toBeVisible();
});
it('disables every preset while a native preview is in flight', () => {
  const onChange = vi.fn();
  render(<OrganizationPresets pattern={defaultPreset.pattern} disabled onChange={onChange} />);
  for (const radio of screen.getAllByRole('radio')) expect(radio).toBeDisabled();
  fireEvent.click(screen.getByRole('radio', { name: 'By month' }));
  expect(onChange).not.toHaveBeenCalled();
});

it('browses all requested groups, preserves current rules until choosing, and opens a custom builder', async () => {
  const onChange = vi.fn();
  render(
    <OrganizationPresets pattern={defaultPreset.pattern} disabled={false} onChange={onChange} />,
  );
  for (const [label, count, name] of [
    ['By location', 4, 'Country → City'],
    ['By tags', 6, 'Tag → Subcategory → Subject'],
    ['By camera', 7, 'Camera model → File format'],
  ] as const) {
    fireEvent.click(screen.getByRole('radio', { name: label }));
    await waitFor(() =>
      expect(
        within(screen.getByRole('radiogroup', { name: 'Folder layout' })).getAllByRole('radio'),
      ).toHaveLength(count),
    );
    expect(onChange).not.toHaveBeenCalled();
    expect(screen.getByRole('radio', { name })).toBeVisible();
  }
  fireEvent.click(screen.getByRole('radio', { name: 'Camera model → File format' }));
  await waitFor(() =>
    expect(onChange).toHaveBeenCalledWith('{camera}/{file_format}/{original_stem}.{ext}'),
  );
  fireEvent.click(screen.getByRole('radio', { name: 'Custom' }));
  expect(await screen.findByRole('combobox', { name: 'Folder levels' })).toBeVisible();
});
