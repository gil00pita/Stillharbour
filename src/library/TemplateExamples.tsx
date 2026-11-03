import { Button, Input } from '../components/ui/controls';
import type { TemplateSample } from '../bindings';

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
    <fieldset disabled={disabled}>
      <legend>Template examples</legend>
      <p className="help">
        Test up to four independent examples when saving. Enter portable source names with / between
        folders and capture-local dates such as 2026-10-31T14:30:00, without a timezone. Examples
        use no files and do not check destination collisions or companions.
      </p>
      {samples.map((sample, index) => (
        <fieldset key={index}>
          <legend>Example {index + 1}</legend>
          <label>
            Example {index + 1} source name
            <Input
              maxLength={256}
              value={sample.relativePath}
              onChange={(e) => update(index, { relativePath: e.target.value })}
            />
          </label>
          <label>
            Example {index + 1} capture date (optional)
            <Input
              maxLength={32}
              value={sample.capture ?? ''}
              onChange={(e) => update(index, { capture: e.target.value || null })}
            />
          </label>
          <label>
            Example {index + 1} sequence
            <Input
              type="number"
              min={1}
              max={240}
              step={1}
              value={Number.isNaN(sample.sequence) ? '' : sample.sequence}
              onChange={(e) => update(index, { sequence: e.target.valueAsNumber })}
            />
          </label>
          <label>
            Example {index + 1} expected target
            <Input
              maxLength={1024}
              value={sample.expectedTarget}
              onChange={(e) => update(index, { expectedTarget: e.target.value })}
            />
          </label>
          <Button type="button" onClick={() => onChange(samples.filter((_, i) => i !== index))}>
            Remove example {index + 1}
          </Button>
        </fieldset>
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
    </fieldset>
  );
}
