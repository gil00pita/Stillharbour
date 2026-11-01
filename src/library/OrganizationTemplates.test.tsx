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
        samples: [],
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

it('submits edited examples, keeps a rejected example for correction and restores retained examples', async () => {
  const sample = {
    relativePath: 'Family/IMG.JPG',
    capture: null,
    sequence: 1,
    expectedTarget: 'IMG.JPG',
  };
  const retained = {
    ...saved,
    policies: {
      ...saved.policies,
      sampleInputs: [JSON.stringify(sample)],
      sampleOutputs: ['IMG.JPG'],
    },
  };
  vi.mocked(libraryData)
    .mockResolvedValueOnce([retained])
    .mockRejectedValueOnce({ message: 'Example 1 does not match. The template produces: IMG.JPG' })
    .mockResolvedValueOnce({ ...retained, reference: { ...saved.reference, version: 2 } });
  render(<OrganizationTemplates template={template} onLoad={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Load saved templates' }));
  await screen.findByRole('option', { name: 'Family <year> · version 1' });
  fireEvent.change(screen.getByLabelText('Saved template'), { target: { value: 'template-id' } });
  expect(screen.getByLabelText('Example 1 source name')).toHaveValue('Family/IMG.JPG');
  fireEvent.change(screen.getByLabelText('Example 1 expected target'), {
    target: { value: 'wrong.JPG' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save new version' }));
  expect(await screen.findByRole('alert')).toHaveTextContent('does not match');
  expect(screen.getByLabelText('Example 1 expected target')).toHaveValue('wrong.JPG');
  expect(vi.mocked(libraryData).mock.calls[1]?.[0]).toMatchObject({
    save: { samples: [{ ...sample, expectedTarget: 'wrong.JPG' }] },
  });
  fireEvent.change(screen.getByLabelText('Example 1 expected target'), {
    target: { value: 'IMG.JPG' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Save new version' }));
  await screen.findByText(/Retained version 2/);
  fireEvent.click(screen.getByRole('button', { name: 'Remove example 1' }));
  expect(screen.queryByLabelText('Example 1 source name')).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Use retained version' }));
  expect(screen.getByLabelText('Example 1 source name')).toHaveValue('Family/IMG.JPG');
});

it('bounds examples and rejects malformed retained example text without loading it', async () => {
  const onLoad = vi.fn();
  vi.mocked(libraryData).mockResolvedValueOnce([
    {
      ...saved,
      policies: {
        ...saved.policies,
        sampleInputs: ['{"relativePath":"<script>"}'],
        sampleOutputs: ['x'],
      },
    },
  ]);
  render(<OrganizationTemplates template={template} onLoad={onLoad} />);
  fireEvent.click(screen.getByRole('button', { name: 'Load saved templates' }));
  await screen.findByRole('option', { name: 'Family <year> · version 1' });
  fireEvent.change(screen.getByLabelText('Saved template'), { target: { value: 'template-id' } });
  expect(screen.getByRole('alert')).toHaveTextContent('invalid');
  expect(onLoad).not.toHaveBeenCalled();
  for (let i = 0; i < 4; i++) fireEvent.click(screen.getByRole('button', { name: 'Add example' }));
  expect(screen.getByRole('button', { name: 'Add example' })).toBeDisabled();
  expect(screen.getAllByText(/expected target/)).toHaveLength(4);
  expect(document.querySelector('script')).toBeNull();
});

it('explains an empty sequence without sending a non-finite number through IPC', async () => {
  vi.mocked(libraryData).mockResolvedValueOnce([]);
  render(<OrganizationTemplates template={template} onLoad={vi.fn()} />);
  fireEvent.click(screen.getByRole('button', { name: 'Load saved templates' }));
  fireEvent.change(await screen.findByLabelText('Template name'), {
    target: { value: 'Examples' },
  });
  fireEvent.click(screen.getByRole('button', { name: 'Add example' }));
  fireEvent.change(screen.getByLabelText('Example 1 sequence'), { target: { value: '' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save template' }));
  expect(screen.getByRole('alert')).toHaveTextContent('whole-number sequence');
  expect(libraryData).toHaveBeenCalledOnce();
});
