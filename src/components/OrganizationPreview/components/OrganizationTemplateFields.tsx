import { Field, Box, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { OrganizationPresets } from '@/components/OrganizationPresets';
import { SegmentedControl } from '@/components/SegmentedControl';
import type { OrganizationTemplate } from '../../../bindings';

export function OrganizationTemplateFields({
  template,
  busy,
  onChange: update,
  submitLabel = 'Build preview',
}: {
  template: OrganizationTemplate;
  busy: boolean;
  submitLabel?: string;
  onChange: (patch: Partial<OrganizationTemplate>) => void;
}) {
  return (
    <chakra.fieldset disabled={busy}>
      <OrganizationPresets
        pattern={template.pattern}
        disabled={busy}
        onChange={(pattern) =>
          update({ pattern, ...(pattern.includes('{event}') ? {} : { event: '' }) })
        }
      />
      {template.pattern.includes('{event}') && (
        <Field.Root asChild unstyled required>
          <label>
            <Field.Label asChild unstyled>
              <span>Event name</span>
            </Field.Label>
            <Input
              required
              maxLength={128}
              value={template.event}
              placeholder="For example, Lisbon-Trip"
              onChange={(e) => update({ event: e.target.value })}
            />
          </label>
        </Field.Root>
      )}
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
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
        id="organization-token-help"
      >
        Tokens:{' '}
        {
          '{capture:%Y-%m-%d}, {quarter}, {country}, {city}, {tag}, {brand}, {model}, {original_stem}, {ext}, {sequence:04}'
        }
        . Keep the original extension. Unsafe names are blocked rather than silently renamed.
      </Text>
      <Box
        className="organization-options"
        css={{
          display: 'grid',
          gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
          gap: '16px',
          '@media (max-width: 760px)': { gridTemplateColumns: 'minmax(0, 1fr)' },
        }}
      >
        <SegmentedControl
          label="When capture date is missing"
          value={template.missingDate}
          disabled={busy}
          choices={[
            { value: 'undated', label: 'Use Undated folder' },
            { value: 'block', label: 'Block photo' },
            ...(template.missingDate === 'skip'
              ? [{ value: 'skip' as const, label: 'Skip photo (preview only)' }]
              : []),
          ]}
          onChange={(missingDate) => update({ missingDate })}
        />
        <SegmentedControl
          label="Filename stem case"
          value={template.filenameStemCase ?? 'preserve'}
          disabled={busy}
          choices={[
            { value: 'preserve', label: 'Preserve' },
            { value: 'lowercase', label: 'lowercase' },
            { value: 'uppercase', label: 'UPPERCASE' },
          ]}
          onChange={(filenameStemCase) => update({ filenameStemCase })}
        />
      </Box>
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
        Stem case applies to proposed filenames, including Undated names. Folder names and extension
        case stay as previewed.
      </Text>
      <Button type="submit" className="primary">
        {busy ? 'Building preview…' : submitLabel}
      </Button>
    </chakra.fieldset>
  );
}
