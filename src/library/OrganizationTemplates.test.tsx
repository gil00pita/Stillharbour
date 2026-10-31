import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, expect, it, vi } from 'vitest';
import { libraryData } from '../ipc';
import { OrganizationTemplates } from './OrganizationTemplates';

vi.mock('../ipc', async (original) => ({
  ...(await original<typeof import('../ipc')>()),
  libraryData: vi.fn(),
}));
const template = { pattern: '{original_stem}.{ext}', event: '', missingDate: 'block' as const };
const saved = {
  reference: { id: 'template-id', version: 1 },
  name: 'Family <year>',
  description: 'Retained',
  createdAt: 'now',
  template,
  policies: {
    schemaVersion: 1,
    enabledTokens: [],
    datePolicy: 'embedded_capture_only',
    filenameCasePolicy: 'preserve',
    collisionPolicy: 'block',
    sequencePolicy: 'root_native_path_then_id',
    sampleInputs: [],
    sampleOutputs: [],
  },
};
beforeEach(() => {
  vi.mocked(libraryData).mockReset();
});
it('loads on demand and saves against the loaded version without silently changing the editor', async () => {
  const onLoad = vi.fn();
  vi.mocked(libraryData)
    .mockResolvedValueOnce([saved])
    .mockResolvedValueOnce({ ...saved, reference: { ...saved.reference, version: 2 } });
  render(<OrganizationTemplates template={template} onLoad={onLoad} />);
  expect(libraryData).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole('button', { name: 'Load saved templates' }));
  await screen.findByRole('option', { name: 'Family <year> · version 1' });
  fireEvent.change(screen.getByLabelText('Saved template'), { target: { value: 'template-id' } });
  expect(onLoad).toHaveBeenCalledWith(template, saved.reference);
  fireEvent.click(screen.getByRole('button', { name: 'Save new version' }));
  await screen.findByText(/Retained version 2/);
  expect(libraryData).toHaveBeenLastCalledWith(
    {
      kind: 'organization_template_save',
      save: {
        previous: saved.reference,
        name: saved.name,
        description: saved.description,
        template,
      },
    },
    'organization_template',
  );
  expect(onLoad).toHaveBeenCalledOnce();
  expect(document.querySelector('year')).toBeNull();
});
it('reports version conflicts and preserves the draft for retry', async () => {
  vi.mocked(libraryData)
    .mockResolvedValueOnce([])
    .mockRejectedValueOnce({ message: 'Reload the changed template.' });
  render(<OrganizationTemplates template={template} onLoad={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Load saved templates' }));
  fireEvent.change(await screen.findByLabelText('Template name'), {
    target: { value: 'My template' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save template' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('Reload');
  expect(screen.getByLabelText('Template name')).toHaveValue('My template');
  await waitFor(() => expect(screen.getByRole('button', { name: 'Save template' })).toBeEnabled());
});
