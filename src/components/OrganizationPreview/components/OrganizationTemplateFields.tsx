import { Field } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import type { OrganizationTemplate } from '../../../bindings';
import { presets } from '../presets';

export function OrganizationTemplateFields({
  template,
  busy,
  onChange: update,
}: {
  template: OrganizationTemplate;
  busy: boolean;
  onChange: (patch: Partial<OrganizationTemplate>) => void;
}) {
  return (
    <fieldset disabled={busy}>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Preset</span>
          </Field.Label>
          <NativeSelect
            value={presets.find((preset) => preset.pattern === template.pattern)?.pattern ?? ''}
            onChange={(event) => {
              if (event.target.value) update({ pattern: event.target.value });
            }}
          >
            <option value="" disabled>
              Custom template
            </option>
            {presets.map((preset) => (
              <option key={preset.name} value={preset.pattern}>
                {preset.name}
              </option>
            ))}
          </NativeSelect>
        </label>
      </Field.Root>
      <Field.Root asChild unstyled>
        <label>
          <Field.Label asChild unstyled>
            <span>Path template</span>
          </Field.Label>
          <Input
            required
            maxLength={1024}
            value={template.pattern}
            onChange={(event) => update({ pattern: event.target.value })}
            spellCheck={false}
            aria-describedby="organization-token-help"
          />
        </label>
      </Field.Root>
      <p className="help" id="organization-token-help">
        Tokens:{' '}
        {'{original_stem}, {original_folder}, {ext}, {event}, {capture:%Y-%m-%d}, {sequence:04}'}.
        Keep the original extension. Unsafe names are blocked rather than silently renamed.
      </p>
      <div className="organization-options">
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Event name</span>
            </Field.Label>
            <Input
              maxLength={128}
              value={template.event}
              onChange={(event) => update({ event: event.target.value })}
            />
          </label>
        </Field.Root>
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>When capture date is missing</span>
            </Field.Label>
            <NativeSelect
              value={template.missingDate}
              onChange={(event) =>
                update({ missingDate: event.target.value === 'block' ? 'block' : 'undated' })
              }
            >
              <option value="undated">Use Undated / original filename</option>
              <option value="block">Block the affected photo</option>
            </NativeSelect>
          </label>
        </Field.Root>
        <Field.Root asChild unstyled>
          <label>
            <Field.Label asChild unstyled>
              <span>Filename stem case</span>
            </Field.Label>
            <NativeSelect
              value={template.filenameStemCase ?? 'preserve'}
              aria-label="Filename stem case"
              onChange={(event) =>
                update({
                  filenameStemCase:
                    event.target.value === 'lowercase'
                      ? 'lowercase'
                      : event.target.value === 'uppercase'
                        ? 'uppercase'
                        : 'preserve',
                })
              }
            >
              <option value="preserve">Preserve</option>
              <option value="lowercase">Lowercase</option>
              <option value="uppercase">Uppercase</option>
            </NativeSelect>
          </label>
        </Field.Root>
      </div>
      <p className="help">
        Stem case applies to proposed filenames, including Undated names. Folder names and extension
        case stay as previewed.
      </p>
      <Button type="submit" className="primary">
        {busy ? 'Building preview…' : 'Build preview'}
      </Button>
    </fieldset>
  );
}
