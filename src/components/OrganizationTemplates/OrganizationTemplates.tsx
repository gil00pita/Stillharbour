import { Field, Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { useEffect, useRef, useState } from 'react';
import type {
  OrganizationTemplate,
  SavedTemplate,
  TemplateRef,
  TemplateSample,
} from '../../bindings';
import { errorMessage, libraryData } from '../../ipc';
import { TemplateExamples } from '@/components/TemplateExamples';
import { retainedExamples } from '../../library/retainedTemplateExamples';

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
    <chakra.section
      className="organization-destination"
      aria-label="Saved templates"
      aria-busy={busy}
    >
      <Heading as="h3" unstyled>
        Saved templates
      </Heading>
      <Button type="button" disabled={busy} onClick={() => void run(false)}>
        Load saved templates
      </Button>
      {templates && (
        <>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Saved template</span>
              </Field.Label>
              <NativeSelect
                value={selected?.reference.id ?? ''}
                disabled={busy}
                onChange={(event) => {
                  const value =
                    templates.find((v) => v.reference.id === event.target.value) ?? null;
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
              </NativeSelect>
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Template name</span>
              </Field.Label>
              <Input
                maxLength={128}
                value={name}
                disabled={busy}
                onChange={(event) => setName(event.target.value)}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Template description</span>
              </Field.Label>
              <Input
                maxLength={1024}
                value={description}
                disabled={busy}
                onChange={(event) => setDescription(event.target.value)}
              />
            </label>
          </Field.Root>
          <TemplateExamples samples={samples} disabled={busy} onChange={setSamples} />
          <Button type="button" disabled={busy || !name.trim()} onClick={() => void run(true)}>
            {selected ? 'Save new version' : 'Save template'}
          </Button>
          {selected && (
            <Button
              type="button"
              disabled={busy || !name.trim()}
              onClick={() => void run(true, true)}
            >
              Save as new template
            </Button>
          )}
          {selected && (
            <Button
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
            </Button>
          )}
          {selected && (
            <Text role="status">
              Retained version {selected.reference.version}. Load it explicitly to use its saved
              settings.
            </Text>
          )}
          <Text className="help">
            Each save retains an immutable version. Editing the preview does not change earlier
            plans.
          </Text>
        </>
      )}
      {error && (
        <Text className="error" role="alert">
          {error}
        </Text>
      )}
    </chakra.section>
  );
}
