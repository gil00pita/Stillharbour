import { Field, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import type { TemplateSample, OrganizationMetadata } from '../../bindings';

const namingFields: { key: keyof OrganizationMetadata; label: string }[] = [
  { key: 'country', label: 'Country' },
  { key: 'region', label: 'Region' },
  { key: 'city', label: 'City' },
  { key: 'place', label: 'Place' },
  { key: 'tag', label: 'Tag' },
  { key: 'subcategory', label: 'Subcategory' },
  { key: 'subject', label: 'Subject' },
  { key: 'cameraType', label: 'Camera type' },
  { key: 'brand', label: 'Brand' },
  { key: 'model', label: 'Model' },
  { key: 'fileFormat', label: 'File format' },
];

export function TemplateExamples({
  samples,
  disabled,
  onChange,
}: {
  samples: TemplateSample[];
  disabled: boolean;
  onChange: (samples: TemplateSample[]) => void;
}) {
  function update(index: number, patch: Partial<TemplateSample>) {
    onChange(samples.map((sample, i) => (i === index ? { ...sample, ...patch } : sample)));
  }
  return (
    <chakra.fieldset disabled={disabled}>
      <chakra.legend>Template examples</chakra.legend>
      <Text className="help">
        Test up to four independent examples when saving. Enter portable source names with / between
        folders and capture-local dates such as 2026-10-31T14:30:00, without a timezone. Examples
        use no files and do not check destination collisions or companions.
      </Text>
      {samples.map((sample, index) => (
        <chakra.fieldset key={index}>
          <chakra.legend>Example {index + 1}</chakra.legend>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Example {index + 1} source name</span>
              </Field.Label>
              <Input
                maxLength={256}
                value={sample.relativePath}
                onChange={(e) => update(index, { relativePath: e.target.value })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Example {index + 1} capture date (optional)</span>
              </Field.Label>
              <Input
                maxLength={32}
                value={sample.capture ?? ''}
                onChange={(e) => update(index, { capture: e.target.value || null })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Example {index + 1} sequence</span>
              </Field.Label>
              <Input
                type="number"
                min={1}
                max={240}
                step={1}
                value={Number.isNaN(sample.sequence) ? '' : sample.sequence}
                onChange={(e) => update(index, { sequence: e.target.valueAsNumber })}
              />
            </label>
          </Field.Root>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Example {index + 1} expected target</span>
              </Field.Label>
              <Input
                maxLength={1024}
                value={sample.expectedTarget}
                onChange={(e) => update(index, { expectedTarget: e.target.value })}
              />
            </label>
          </Field.Root>
          <details>
            <summary>Example location, tag and camera values</summary>
            {namingFields.map((field) => (
              <Field.Root key={field.key} asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>
                      Example {index + 1} {field.label.toLowerCase()}
                    </span>
                  </Field.Label>
                  <Input
                    maxLength={128}
                    value={sample.metadata?.[field.key] ?? ''}
                    onChange={(event) => {
                      const metadata = { ...sample.metadata };
                      if (event.target.value) metadata[field.key] = event.target.value;
                      else delete metadata[field.key];
                      update(index, { metadata });
                    }}
                  />
                </label>
              </Field.Root>
            ))}
          </details>
          <Button type="button" onClick={() => onChange(samples.filter((_, i) => i !== index))}>
            Remove example {index + 1}
          </Button>
        </chakra.fieldset>
      ))}
      <Button
        type="button"
        disabled={samples.length >= 4}
        onClick={() =>
          onChange([
            ...samples,
            { relativePath: '', capture: null, sequence: 1, expectedTarget: '' },
          ])
        }
      >
        Add example
      </Button>
    </chakra.fieldset>
  );
}
