import { RadioCard, useSlotRecipe, Box, Image, Text } from '@chakra-ui/react';
import { useId } from 'react';
import { inspectionChoiceCardsRecipe } from './InspectionChoiceCards.slot-recipe';

export type InspectionChoice<Value extends string> = {
  value: Value;
  label: string;
  description?: string;
  image?: string;
  badge?: string;
};

export function InspectionChoiceCards<Value extends string>({
  label,
  value,
  choices,
  onChange,
  disabled = false,
  presentation = 'compact',
}: {
  label: string;
  value: Value;
  choices: readonly InspectionChoice<Value>[];
  onChange: (value: Value) => void;
  disabled?: boolean;
  presentation?: 'illustrated' | 'compact' | 'inline';
}) {
  const id = useId();
  const styles = useSlotRecipe({ recipe: inspectionChoiceCardsRecipe })({ presentation });
  return (
    <RadioCard.Root
      className="inspection-choice-group"
      name={id}
      value={value}
      disabled={disabled}
      css={styles.root}
      onValueChange={({ value: next }) => {
        const choice = choices.find((item) => item.value === next);
        if (choice && !disabled) onChange(choice.value);
      }}
    >
      <RadioCard.Label css={styles.label} className="inspection-choice-label">
        {label}
      </RadioCard.Label>
      <Box className="inspection-choice-grid">
        {choices.map((choice) => (
          <RadioCard.Item
            key={choice.value}
            value={choice.value}
            className="inspection-choice-card"
            css={styles.item}
          >
            <RadioCard.ItemHiddenInput
              aria-describedby={choice.description ? `${id}-${choice.value}` : undefined}
            />
            <RadioCard.ItemControl css={styles.itemControl}>
              {choice.image && (
                <Image
                  src={choice.image}
                  alt=""
                  htmlWidth={640}
                  htmlHeight={427}
                  decoding="async"
                />
              )}
              <RadioCard.ItemContent css={styles.itemContent}>
                <RadioCard.ItemText css={styles.itemText}>{choice.label}</RadioCard.ItemText>
                {choice.description && (
                  <RadioCard.ItemDescription
                    css={styles.itemDescription}
                    id={`${id}-${choice.value}`}
                  >
                    {choice.description}
                    {choice.badge && (
                      <Text as="span" className="inspection-choice-badge">
                        {choice.badge}
                      </Text>
                    )}
                  </RadioCard.ItemDescription>
                )}
              </RadioCard.ItemContent>
              <RadioCard.ItemIndicator css={styles.itemIndicator} />
            </RadioCard.ItemControl>
          </RadioCard.Item>
        ))}
      </Box>
    </RadioCard.Root>
  );
}
