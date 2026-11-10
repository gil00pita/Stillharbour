import { Box, Field, HStack, SegmentGroup, useSlotRecipe } from '@chakra-ui/react';
import { useId, type ReactNode } from 'react';
import { segmentedControlRecipe } from './SegmentedControl.slot-recipe';

export function SegmentedControl<Value extends string>({
  label,
  value,
  choices,
  disabled,
  onChange,
}: {
  label: string;
  value: Value;
  choices: readonly { value: Value; label: string; icon?: ReactNode }[];
  disabled: boolean;
  onChange: (value: Value) => void;
}) {
  const id = useId();
  const styles = useSlotRecipe({ recipe: segmentedControlRecipe })();
  return (
    <Field.Root disabled={disabled} unstyled display="grid" gap="2" minWidth="0">
      <Field.Label id={`${id}-label`} asChild unstyled>
        <span>{label}</span>
      </Field.Label>
      <SegmentGroup.Root
        className="segmented-control"
        unstyled
        name={id}
        value={value}
        disabled={disabled}
        aria-labelledby={`${id}-label`}
        css={styles.root}
        onValueChange={({ value: next }) => {
          const choice = choices.find((item) => item.value === next);
          if (choice && !disabled) onChange(choice.value);
        }}
      >
        {choices.map((choice) => (
          <SegmentGroup.Item
            key={choice.value}
            value={choice.value}
            css={[
              styles.item,
              choice.icon ? { flex: '1 0 auto', paddingInline: { base: '1', md: '1.5' } } : {},
            ]}
          >
            <SegmentGroup.ItemText css={styles.itemText}>
              {choice.icon ? (
                <HStack
                  as="span"
                  gap={{ base: '1', md: '1.5' }}
                  justify="center"
                  minW="0"
                  whiteSpace="nowrap"
                >
                  <Box as="span" display="inline-flex" flexShrink="0" aria-hidden="true">
                    {choice.icon}
                  </Box>
                  <span>{choice.label}</span>
                </HStack>
              ) : (
                choice.label
              )}
            </SegmentGroup.ItemText>
            <SegmentGroup.ItemHiddenInput />
          </SegmentGroup.Item>
        ))}
      </SegmentGroup.Root>
    </Field.Root>
  );
}
