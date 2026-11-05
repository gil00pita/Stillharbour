import { RadioCard } from '@chakra-ui/react';
import { useId } from 'react';

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
  return (
    <RadioCard.Root
      className="inspection-choice-group"
      name={id}
      value={value}
      disabled={disabled}
      presentation={presentation}
      onValueChange={({ value: next }) => {
        const choice = choices.find((item) => item.value === next);
        if (choice && !disabled) onChange(choice.value);
      }}
    >
      <RadioCard.Label className="inspection-choice-label">{label}</RadioCard.Label>
      <div className="inspection-choice-grid">
        {choices.map((choice) => (
          <RadioCard.Item
            key={choice.value}
            value={choice.value}
            className="inspection-choice-card"
          >
            <RadioCard.ItemHiddenInput
              aria-describedby={choice.description ? `${id}-${choice.value}` : undefined}
            />
            <RadioCard.ItemControl>
              {choice.image && (
                <img src={choice.image} alt="" width="640" height="427" decoding="async" />
              )}
              <RadioCard.ItemContent>
                <RadioCard.ItemText>{choice.label}</RadioCard.ItemText>
                {choice.description && (
                  <RadioCard.ItemDescription id={`${id}-${choice.value}`}>
                    {choice.description}
                    {choice.badge && (
                      <span className="inspection-choice-badge">{choice.badge}</span>
                    )}
                  </RadioCard.ItemDescription>
                )}
              </RadioCard.ItemContent>
              <RadioCard.ItemIndicator />
            </RadioCard.ItemControl>
          </RadioCard.Item>
        ))}
      </div>
    </RadioCard.Root>
  );
}
