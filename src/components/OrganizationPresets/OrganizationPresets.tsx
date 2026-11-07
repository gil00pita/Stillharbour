import { Box, RadioCard, Text, useSlotRecipe } from '@chakra-ui/react';
import { useId, useState } from 'react';
import { SegmentedControl } from '@/components/SegmentedControl';
import { StructurePathBuilder } from '@/components/StructurePathBuilder';
import { presets, type LayoutGroup } from './presets';
import { organizationPresetsRecipe } from './OrganizationPresets.slot-recipe';
import { PresetIllustration } from './components/PresetIllustration';

type LayoutTab = LayoutGroup | 'custom';
export function OrganizationPresets({
  pattern,
  disabled,
  onChange,
}: {
  pattern: string;
  disabled: boolean;
  onChange: (pattern: string) => void;
}) {
  const id = useId();
  const styles = useSlotRecipe({ recipe: organizationPresetsRecipe })();
  const selected = presets.find((preset) => preset.pattern === pattern);
  const [tab, setTab] = useState<LayoutTab>(selected?.group ?? 'custom');
  const [previousPattern, setPreviousPattern] = useState(pattern);
  // Loading an external saved rule reveals its category; browsing a category
  // alone does not change the current rule. Custom edits stay in the builder.
  if (previousPattern !== pattern) {
    setPreviousPattern(pattern);
    if (tab !== 'custom') setTab(selected?.group ?? 'custom');
  }
  return (
    <Box className="structure-layout-picker">
      <SegmentedControl
        label="Layout type"
        value={tab}
        disabled={disabled}
        choices={[
          { value: 'date', label: 'By date' },
          { value: 'location', label: 'By location' },
          { value: 'tags', label: 'By tags' },
          { value: 'camera', label: 'By camera' },
          { value: 'custom', label: 'Custom' },
        ]}
        onChange={setTab}
      />
      {tab === 'custom' ? (
        <StructurePathBuilder pattern={pattern} disabled={disabled} onChange={onChange} />
      ) : (
        <>
          <RadioCard.Root
            unstyled
            name={id}
            value={selected?.pattern ?? ''}
            disabled={disabled}
            className="organization-presets"
            css={styles.root}
            onValueChange={({ value }) => {
              const preset = presets.find((item) => item.pattern === value);
              if (preset && !disabled) onChange(preset.pattern);
            }}
          >
            <RadioCard.Label css={styles.label}>Folder layout</RadioCard.Label>
            <Box css={styles.grid}>
              {presets
                .filter((preset) => preset.group === tab)
                .map((preset) => (
                  <RadioCard.Item key={preset.pattern} value={preset.pattern} css={styles.item}>
                    <RadioCard.ItemHiddenInput />
                    <RadioCard.ItemControl css={styles.control}>
                      <PresetIllustration paths={preset.example} css={styles.illustration} />
                      <RadioCard.ItemContent css={styles.content}>
                        <RadioCard.ItemText css={styles.text}>{preset.name}</RadioCard.ItemText>
                        <RadioCard.ItemDescription css={styles.description}>
                          {preset.description}
                        </RadioCard.ItemDescription>
                      </RadioCard.ItemContent>
                      <RadioCard.ItemIndicator css={styles.indicator} />
                    </RadioCard.ItemControl>
                  </RadioCard.Item>
                ))}
            </Box>
          </RadioCard.Root>
          {(!selected || selected.group !== tab) && (
            <Text className="help">
              Current layout: {selected?.name ?? 'Custom'}. Choose a layout above to replace it.
            </Text>
          )}
        </>
      )}
      {tab === 'location' && (
        <Text className="help">
          Uses embedded country, region, city and place names already indexed in the catalog. GPS is
          not used to infer names. Missing or conflicting required fields block the affected photo.
        </Text>
      )}
      {tab === 'tags' && (
        <Text className="help">
          Uses catalog tags and their category parents. Required levels must form an unambiguous
          hierarchy; unrelated tags are not assigned an arbitrary priority.
        </Text>
      )}
      {tab === 'camera' && (
        <Text className="help">
          Uses observed camera metadata. Camera type needs an explicit value; make/model alone do
          not identify it. Format folders use the indexed format or file extension, with known raw
          suffixes grouped as RAW.
        </Text>
      )}
    </Box>
  );
}
