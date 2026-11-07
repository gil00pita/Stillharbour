import { Box, RadioCard, Text, useSlotRecipe } from '@chakra-ui/react';
import { useId, useState } from 'react';
import { SegmentedControl } from '@/components/SegmentedControl';
import { StructurePathBuilder } from '@/components/StructurePathBuilder';
import { presets, type LayoutGroup, type OrganizationPreset } from './presets';
import { organizationPresetsRecipe } from './OrganizationPresets.slot-recipe';
import { PresetIllustration } from './components/PresetIllustration';
import { pathParts, withFolders } from '@/components/StructurePathBuilder';

export type LayoutTab = LayoutGroup | 'custom';

const categoryHelp: Partial<Record<LayoutTab, string>> = {
  location:
    'Uses embedded country, region, city and place names already indexed in the catalog. GPS is not used to infer names. Missing or conflicting required fields block the affected photo.',
  tags: 'Uses catalog tags and their category parents. Required levels must form an unambiguous hierarchy; unrelated tags are not assigned an arbitrary priority.',
  camera:
    'Uses observed camera metadata. Camera type needs an explicit value; make/model alone do not identify it. Format folders use the indexed format or file extension, with known raw suffixes grouped as RAW.',
};

function matchesPattern(preset: OrganizationPreset, pattern: string, folderOnly: boolean) {
  return folderOnly
    ? pathParts(preset.pattern).folders.join('/') === pathParts(pattern).folders.join('/')
    : preset.pattern === pattern;
}

function presetDescription(preset: OrganizationPreset, folderOnly: boolean) {
  return folderOnly && ['By day', 'By month', 'Date & time'].includes(preset.name)
    ? `${pathParts(preset.pattern).folders.length} folder levels · your filename choice`
    : preset.description;
}

export function OrganizationPresets({
  pattern,
  disabled,
  onChange,
  folderOnly = false,
  category,
  onCategoryChange,
}: {
  pattern: string;
  disabled: boolean;
  onChange: (pattern: string) => void;
  folderOnly?: boolean;
  category?: LayoutTab;
  onCategoryChange?: (category: LayoutTab) => void;
}) {
  const id = useId();
  const styles = useSlotRecipe({ recipe: organizationPresetsRecipe })();
  const selected = presets.find((preset) => matchesPattern(preset, pattern, folderOnly));
  const [localTab, setTab] = useState<LayoutTab>(selected?.group ?? 'custom');
  const tab = category ?? localTab;
  const [previousPattern, setPreviousPattern] = useState(pattern);
  // Loading an external saved rule reveals its category; browsing a category
  // alone does not change the current rule. Custom edits stay in the builder.
  if (previousPattern !== pattern) {
    setPreviousPattern(pattern);
    if (!category && tab !== 'custom') setTab(selected?.group ?? 'custom');
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
        onChange={(value) => {
          setTab(value);
          onCategoryChange?.(value);
        }}
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
              if (preset && !disabled)
                onChange(
                  folderOnly
                    ? withFolders(pathParts(preset.pattern).folders, pathParts(pattern).filename)
                    : preset.pattern,
                );
            }}
          >
            <RadioCard.Label css={styles.label}>Folder layout</RadioCard.Label>
            <Box css={styles.grid} data-compact={folderOnly || undefined}>
              {presets
                .filter((preset) => preset.group === tab)
                .map((preset) => (
                  <RadioCard.Item key={preset.pattern} value={preset.pattern} css={styles.item}>
                    <RadioCard.ItemHiddenInput />
                    <RadioCard.ItemControl
                      css={styles.control}
                      data-compact={folderOnly || undefined}
                    >
                      <PresetIllustration
                        paths={
                          folderOnly
                            ? [...preset.example.slice(0, -1), 'Photo filename']
                            : preset.example
                        }
                        css={styles.illustration}
                        data-compact={folderOnly || undefined}
                      />
                      <RadioCard.ItemContent css={styles.content}>
                        <RadioCard.ItemText css={styles.text}>{preset.name}</RadioCard.ItemText>
                        <RadioCard.ItemDescription css={styles.description}>
                          {presetDescription(preset, folderOnly)}
                        </RadioCard.ItemDescription>
                      </RadioCard.ItemContent>
                      <RadioCard.ItemIndicator css={styles.indicator} />
                    </RadioCard.ItemControl>
                  </RadioCard.Item>
                ))}
            </Box>
          </RadioCard.Root>
          {(!selected || selected.group !== tab) && (
            <Text
              className="help"
              css={{
                fontSize: '13px',
                color: '{colors.fg.muted}',
                lineHeight: '20px',
                margin: '8px 0',
              }}
            >
              Current layout: {selected?.name ?? 'Custom'}. Choose a layout above to replace it.
            </Text>
          )}
        </>
      )}
      {categoryHelp[tab] && (
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          {categoryHelp[tab]}
        </Text>
      )}
    </Box>
  );
}
