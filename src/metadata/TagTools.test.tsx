import { beforeEach, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '../test/render';
import { TagTools } from './TagTools';
import { metadata } from './ipc';
import { libraryData } from '../ipc';
vi.mock('./ipc', () => ({ metadata: vi.fn() }));
vi.mock('../ipc', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
beforeEach(() => {
  vi.mocked(metadata)
    .mockReset()
    .mockImplementation(async (request) =>
      request.kind === 'tag_history'
        ? ([] as never)
        : ({
            id: 'review',
            digest: 'digest',
            revision: '9',
            from: 'Family',
            to: 'Travel',
            affectedCount: 2,
            merges: true,
          } as never),
    );
  vi.mocked(libraryData)
    .mockReset()
    .mockResolvedValue({ revision: '9' } as never);
});
it('previews an exact tag merge before applying the retained backend plan', async () => {
  render(<TagTools onChanged={async () => {}} onBusy={vi.fn()} />);
  const details = screen.getByText('Rename, merge, remove & undo tags').closest('details')!;
  details.open = true;
  fireEvent(details, new Event('toggle', { bubbles: true }));
  fireEvent.change(await screen.findByLabelText('Exact tag to change'), {
    target: { value: 'Family' },
  });
  fireEvent.change(screen.getByLabelText('New or existing exact label'), {
    target: { value: 'Travel' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Preview tag change' }));
  expect(await screen.findByText(/2 media · merge/)).toBeVisible();
  expect(metadata).not.toHaveBeenCalledWith(
    expect.objectContaining({ kind: 'tag_edit_apply' }),
    expect.anything(),
  );
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Apply reviewed tag change' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Apply reviewed tag change' }));
  await waitFor(() =>
    expect(metadata).toHaveBeenCalledWith(
      { kind: 'tag_edit_apply', id: 'review', digest: 'digest' },
      'tag_history_entry',
    ),
  );
});
it('keeps undo disabled until the changed catalog and fresh history finish reloading', async () => {
  let applied = false;
  let releaseRefresh!: () => void;
  const refresh = new Promise<void>((resolve) => {
    releaseRefresh = resolve;
  });
  const onChanged = vi.fn().mockResolvedValue(undefined);
  onChanged.mockImplementationOnce(async () => {}).mockImplementationOnce(() => refresh);
  vi.mocked(metadata).mockImplementation(async (request) => {
    if (request.kind === 'tag_history')
      return [
        { id: applied ? 'new-history' : 'old-history', affectedCount: 1, createdAt: '2026-11-02' },
      ] as never;
    if (request.kind === 'tag_edit_apply') {
      applied = true;
      return { id: 'new-history' } as never;
    }
    return {
      id: 'review',
      digest: 'digest',
      revision: '9',
      from: 'Family',
      to: 'Travel',
      affectedCount: 1,
      merges: false,
    } as never;
  });
  vi.mocked(libraryData).mockImplementation(
    async () => ({ revision: applied ? '10' : '9' }) as never,
  );
  render(<TagTools onChanged={onChanged} onBusy={vi.fn()} />);
  const details = screen.getByText('Rename, merge, remove & undo tags').closest('details')!;
  details.open = true;
  fireEvent(details, new Event('toggle', { bubbles: true }));
  fireEvent.change(await screen.findByLabelText('Exact tag to change'), {
    target: { value: 'Family' },
  });
  fireEvent.change(screen.getByLabelText('New or existing exact label'), {
    target: { value: 'Travel' },
  });
  await waitFor(() =>
    expect(screen.getByRole('button', { name: 'Preview tag change' })).toBeEnabled(),
  );
  fireEvent.click(screen.getByRole('button', { name: 'Preview tag change' }));
  const apply = await screen.findByRole('button', { name: 'Apply reviewed tag change' });
  await waitFor(() => expect(apply).toBeEnabled());
  fireEvent.click(apply);
  await waitFor(() => expect(onChanged).toHaveBeenCalledTimes(2));
  expect(screen.getByRole('button', { name: 'Undo tag change old-hist' })).toBeDisabled();
  releaseRefresh();
  const undo = await screen.findByRole('button', { name: 'Undo tag change new-hist' });
  await waitFor(() => expect(undo).toBeEnabled());
  fireEvent.click(undo);
  await waitFor(() =>
    expect(metadata).toHaveBeenCalledWith(
      { kind: 'tag_undo', id: 'new-history', expected_revision: '10' },
      'tag_history_entry',
    ),
  );
});
