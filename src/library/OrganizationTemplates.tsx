import { useEffect, useRef, useState } from 'react';
import type { OrganizationTemplate, SavedTemplate, TemplateRef, TemplateSample } from '../bindings';
import { errorMessage, libraryData } from '../ipc';
import { TemplateExamples } from './TemplateExamples';
import { retainedExamples } from './retainedTemplateExamples';

export function OrganizationTemplates({
  template,
  onLoad,
}: {
  template: OrganizationTemplate;
  onLoad: (template: OrganizationTemplate, reference: TemplateRef) => void;
}) {
  const [templates, setTemplates] = useState<SavedTemplate[] | null>(null);
  const [selected, setSelected] = useState<SavedTemplate | null>(null);
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [samples, setSamples] = useState<TemplateSample[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const request = useRef(0);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  async function run(save: boolean, fork = false) {
    if (busy) return;
    if (
      save &&
      samples.some((s) => !Number.isInteger(s.sequence) || s.sequence < 1 || s.sequence > 240)
    ) {
      setError('Each example needs a whole-number sequence from 1–240.');
      return;
    }
    const id = ++request.current;
    setBusy(true);
    setError(null);
    try {
      if (save) {
        const saved = await libraryData(
          {
            kind: 'organization_template_save',
            save: {
              previous: fork ? null : (selected?.reference ?? null),
              name,
              description,
              template,
              samples,
            },
          },
          'organization_template',
        );
        if (id !== request.current) return;
        setSelected(saved);
        setName(saved.name);
        setDescription(saved.description);
        setTemplates((values) => [
          saved,
          ...(values ?? []).filter((v) => v.reference.id !== saved.reference.id),
        ]);
        // Saving retains the submitted template even if the editor changed while awaiting IPC.
      } else {
        const values = await libraryData(
          { kind: 'organization_templates' },
          'organization_templates',
        );
        if (id === request.current) setTemplates(values);
      }
    } catch (error) {
      if (id === request.current) setError(errorMessage(error));
    } finally {
      if (id === request.current) setBusy(false);
    }
  }
  return (
    <section className="organization-destination" aria-label="Saved templates" aria-busy={busy}>
      <h3>Saved templates</h3>
      <button type="button" disabled={busy} onClick={() => void run(false)}>
        Load saved templates
      </button>
      {templates && (
        <>
          <label>
            Saved template
            <select
              value={selected?.reference.id ?? ''}
              disabled={busy}
              onChange={(event) => {
                const value = templates.find((v) => v.reference.id === event.target.value) ?? null;
                try {
                  setSamples(value ? retainedExamples(value) : []);
                  setError(null);
                } catch (error) {
                  setError(errorMessage(error));
                  return;
                }
                setSelected(value);
                setName(value?.name ?? '');
                setDescription(value?.description ?? '');
                if (value) onLoad(value.template, value.reference);
              }}
            >
              <option value="">New template</option>
              {templates.map((v) => (
                <option key={v.reference.id} value={v.reference.id}>
                  {v.name} · version {v.reference.version}
                </option>
              ))}
            </select>
          </label>
          <label>
            Template name
            <input
              maxLength={128}
              value={name}
              disabled={busy}
              onChange={(event) => setName(event.target.value)}
            />
          </label>
          <label>
            Template description
            <input
              maxLength={1024}
              value={description}
              disabled={busy}
              onChange={(event) => setDescription(event.target.value)}
            />
          </label>
          <TemplateExamples samples={samples} disabled={busy} onChange={setSamples} />
          <button type="button" disabled={busy || !name.trim()} onClick={() => void run(true)}>
            {selected ? 'Save new version' : 'Save template'}
          </button>
          {selected && (
            <button
              type="button"
              disabled={busy || !name.trim()}
              onClick={() => void run(true, true)}
            >
              Save as new template
            </button>
          )}
          {selected && (
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                try {
                  setSamples(retainedExamples(selected));
                  setError(null);
                  onLoad(selected.template, selected.reference);
                } catch (error) {
                  setError(errorMessage(error));
                }
              }}
            >
              Use retained version
            </button>
          )}
          {selected && (
            <p role="status">
              Retained version {selected.reference.version}. Load it explicitly to use its saved
              settings.
            </p>
          )}
          <p className="help">
            Each save retains an immutable version. Editing the preview does not change earlier
            plans.
          </p>
        </>
      )}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
