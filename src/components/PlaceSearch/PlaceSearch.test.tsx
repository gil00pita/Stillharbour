import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../../test/render';
import { libraryData } from '../../ipc';
import { PlaceSearch } from './PlaceSearch';
import type { PlaceSearchReply, PlaceSummary } from '../../bindings';

vi.mock('../../ipc', async (original) => ({
  ...(await original<typeof import('../../ipc')>()),
  libraryData: vi.fn(),
}));
const london: PlaceSummary = {
  id: 'places-v1:city:2643743',
  label: 'London, England, United Kingdom',
  kind: 'city',
  area: 'Within 25 km of the city centre',
  bounds: { west: -0.5, east: 0.3, south: 51.2, north: 51.8 },
};
const portugal: PlaceSummary = {
  id: 'places-v1:country:PRT',
  label: 'Portugal',
  kind: 'country',
  area: 'Approximate country or territory boundaries',
  bounds: { west: -31.3, east: -6.1, south: 30.0, north: 42.2 },
};
const changed = vi.fn();
function reply(
  matches: PlaceSummary[] = [],
  selected: PlaceSummary | null = null,
): PlaceSearchReply {
  return {
    matches,
    selected,
    more: false,
    credits: 'GeoNames · CC BY 4.0; Natural Earth · public domain',
  };
}
beforeEach(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  changed.mockReset();
  vi.mocked(libraryData)
    .mockReset()
    .mockImplementation(async (request) => {
      if (request.kind !== 'places') throw new Error('Unexpected request');
      return reply(
        request.text ? [london, portugal] : [],
        request.selected_id ? london : null,
      ) as never;
    });
});

it('offers labelled geographic matches and applies only the explicitly chosen place', async () => {
  render(<PlaceSearch onChange={changed} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  const input = await screen.findByRole('searchbox', { name: 'City or country' });
  await waitFor(() => expect(input).toHaveFocus());
  fireEvent.change(input, { target: { value: 'London' } });
  const choose = await screen.findByRole('button', { name: /London, England, United Kingdom/ });
  expect(choose).toHaveTextContent('Within 25 km');
  expect(screen.getByText(/Only photos with indexed GPS/)).toBeVisible();
  expect(changed).not.toHaveBeenCalled();
  fireEvent.click(choose);
  expect(changed).toHaveBeenCalledWith(london);
  await waitFor(() => expect(screen.getByRole('button', { name: 'Search places' })).toHaveFocus());
});

it('clears an existing place and can close without changing the query', async () => {
  render(<PlaceSearch selectedId={london.id} onChange={changed} />);
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  expect(await screen.findByText(`Selected: ${london.label}`)).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Close place search' }));
  expect(changed).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  fireEvent.click(await screen.findByRole('button', { name: 'Clear place filter' }));
  expect(changed).toHaveBeenCalledWith(null);
});

it('reports a failed lookup and retries without applying a fictional place', async () => {
  vi.mocked(libraryData).mockRejectedValueOnce(new Error('Offline index unavailable.'));
  render(<PlaceSearch onChange={changed} />);
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Offline index unavailable.');
  expect(changed).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Retry place search' }));
  expect(await screen.findByText('Enter at least two characters.')).toBeVisible();
});

it('discards a late lookup after another search has completed', async () => {
  let release: ((value: PlaceSearchReply) => void) | undefined;
  vi.mocked(libraryData).mockImplementation(async (request) => {
    if (request.kind !== 'places') throw new Error('Unexpected request');
    if (request.text === 'London')
      return (await new Promise<PlaceSearchReply>((resolve) => {
        release = resolve;
      })) as never;
    return reply(request.text === 'Portugal' ? [portugal] : []) as never;
  });
  render(<PlaceSearch onChange={changed} />);
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  const input = await screen.findByRole('searchbox', { name: 'City or country' });
  fireEvent.change(input, { target: { value: 'London' } });
  await waitFor(() => expect(release).toBeDefined());
  fireEvent.change(input, { target: { value: 'Portugal' } });
  expect(await screen.findByRole('button', { name: /Portugal Approximate/ })).toBeVisible();
  release?.(reply([london]));
  await waitFor(() =>
    expect(screen.queryByRole('button', { name: /London, England/ })).not.toBeInTheDocument(),
  );
  expect(changed).not.toHaveBeenCalled();
});

it('keeps the picker unavailable while catalog actions block filter changes', () => {
  render(<PlaceSearch onChange={changed} disabled />);
  expect(screen.getByRole('button', { name: 'Search places' })).toBeDisabled();
  expect(libraryData).not.toHaveBeenCalled();
});

it('lets an unavailable saved place be cleared without claiming it is still loading', async () => {
  vi.mocked(libraryData).mockRejectedValue(new Error('This place is unavailable.'));
  render(<PlaceSearch selectedId="places-v1:city:missing" onChange={changed} />);
  fireEvent.click(screen.getByRole('button', { name: 'Search places' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('This place is unavailable.');
  expect(screen.getByText('Selected: Place details unavailable')).toBeVisible();
  fireEvent.click(screen.getByRole('button', { name: 'Clear place filter' }));
  expect(changed).toHaveBeenCalledWith(null);
});
