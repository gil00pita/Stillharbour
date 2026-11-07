import { Box, Field, Heading, Text, chakra } from '@chakra-ui/react';
import { useEffect, useRef } from 'react';
import { focusRing } from '../../../theme/focus-ring';
import { OrganizationPresets, type LayoutTab } from '@/components/OrganizationPresets';
import { SegmentedControl } from '@/components/SegmentedControl';
import { Input } from '@/components/Input';
import type { OrganizationTemplate } from '../../../bindings';
import { filenameChoice, withFilename } from '../structure-rules';

export function StructureRuleEditor({
  template,
  category,
  busy,
  onChange,
  onCategoryChange,
}: {
  template: OrganizationTemplate;
  category: LayoutTab;
  busy: boolean;
  onChange: (patch: Partial<OrganizationTemplate>) => void;
  onCategoryChange: (category: LayoutTab) => void;
}) {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
  }, []);
  return (
    <chakra.fieldset disabled={busy} className="structure-rule-editor">
      <Heading as="h2" fontSize="lg" ref={heading} tabIndex={-1} _focusVisible={focusRing}>
        Folder structure
      </Heading>
      <OrganizationPresets
        pattern={template.pattern}
        disabled={busy}
        folderOnly
        category={category}
        onCategoryChange={onCategoryChange}
        onChange={(pattern) =>
          onChange({ pattern, ...(pattern.includes('{event}') ? {} : { event: '' }) })
        }
      />
      {template.pattern.includes('{event}') && (
        <Field.Root required>
          <Field.Label>Event name</Field.Label>
          <Input
            required
            maxLength={128}
            value={template.event}
            placeholder="For example, Lisbon-Trip"
            onChange={(event) => onChange({ event: event.target.value })}
          />
        </Field.Root>
      )}
      <Box className="structure-naming">
        <Heading as="h2" fontSize="lg" mb="4">
          File naming
        </Heading>
        <SegmentedControl
          label="Filename format"
          value={filenameChoice(template.pattern)}
          disabled={busy}
          choices={[
            { value: 'original', label: 'Original' },
            { value: 'date_original', label: 'Date + original' },
            { value: 'sequence', label: 'Sequence number' },
          ]}
          onChange={(choice) => {
            if (choice !== 'custom') onChange({ pattern: withFilename(template.pattern, choice) });
          }}
        />
        {filenameChoice(template.pattern) === 'custom' && (
          <Text className="help">
            Custom filename pattern. Choose an option above to replace it.
          </Text>
        )}
        <Text className="help">
          Sequence numbers follow the entire library’s stable preview order. The original extension
          stays unchanged.
        </Text>
      </Box>
      <SegmentedControl
        label="When capture date is missing"
        value={template.missingDate}
        disabled={busy}
        choices={[
          { value: 'undated', label: 'Use Undated folder' },
          { value: 'skip', label: 'Skip photo' },
          ...(template.missingDate === 'block'
            ? [{ value: 'block' as const, label: 'Block photo (saved rule)' }]
            : []),
        ]}
        onChange={(missingDate) => onChange({ missingDate })}
      />
      <Text className="help">
        Applies when the rule needs a capture date. Undated uses the original filename; skipped
        items stay in place.
      </Text>
      <SegmentedControl
        label="Filename stem case"
        value={template.filenameStemCase ?? 'preserve'}
        disabled={busy}
        choices={[
          { value: 'preserve', label: 'Preserve' },
          { value: 'lowercase', label: 'lowercase' },
          { value: 'uppercase', label: 'UPPERCASE' },
        ]}
        onChange={(filenameStemCase) => onChange({ filenameStemCase })}
      />
      <Text className="help">
        Letter case changes the filename stem only, including Undated names.
      </Text>
      <chakra.details className="structure-advanced">
        <chakra.summary>Advanced path rule</chakra.summary>
        <Field.Root mt="3">
          <Field.Label>Path template</Field.Label>
          <Input
            name="structure-pattern"
            required
            maxLength={1024}
            value={template.pattern}
            spellCheck={false}
            onChange={(event) => onChange({ pattern: event.target.value })}
          />
          <Field.HelperText>
            Use folder chips for standard parts. The native preview validates paths, extensions and
            conflicts before any future execution.
          </Field.HelperText>
        </Field.Root>
      </chakra.details>
    </chakra.fieldset>
  );
}
