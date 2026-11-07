import { Field, SegmentGroup, useSlotRecipe } from '@chakra-ui/react';
import { useId } from 'react';
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
  choices: readonly { value: Value; label: string }[];
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
          <SegmentGroup.Item key={choice.value} value={choice.value} css={styles.item}>
            <SegmentGroup.ItemText css={styles.itemText}>{choice.label}</SegmentGroup.ItemText>
            <SegmentGroup.ItemHiddenInput />
          </SegmentGroup.Item>
        ))}
      </SegmentGroup.Root>
    </Field.Root>
  );
}
