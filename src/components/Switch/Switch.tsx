import { chakra } from '@chakra-ui/react';
import { Text } from '@chakra-ui/react';
export function Switch({
  label,
  checked,
  onChange,
  disabled = false,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <chakra.label
      className="switch-field"
      css={{
        flexDirection: 'row',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '14px',
        minHeight: '38px',
        cursor: 'pointer',
      }}
    >
      <Text as="span">{label}</Text>
      <chakra.input
        className="switch-control"
        css={{
          'input&': {
            appearance: 'none',
            position: 'relative',
            width: '36px',
            height: '22px',
            padding: '0',
            margin: '0',
            flex: '0 0 36px',
            border: '1px solid {colors.border.control}',
            borderRadius: '12px',
            background: '{colors.border.button}',
            cursor: 'pointer',
          },
          'input&::before': {
            content: "''",
            position: 'absolute',
            top: '3px',
            left: '3px',
            width: '14px',
            height: '14px',
            borderRadius: '50%',
            background: 'var(--stillharbour-colors-fg, {colors.fg})',
          },
          'input&:checked': {
            borderColor: '{colors.brand.solid}',
            background: '{colors.brand.solid}',
          },
          'input&:checked::before': {
            transform: 'translateX(14px)',
            background: '{colors.brand.contrast}',
          },
          'input&:disabled': { opacity: '0.55', cursor: 'default' },
          '@media (forced-colors: active)': {
            'input&': { background: 'Canvas', borderColor: 'CanvasText' },
            'input&:checked': { background: 'Highlight' },
            'input&::before': { background: 'CanvasText' },
            'input&:checked::before': { background: 'CanvasText' },
          },
        }}
        type="checkbox"
        role="switch"
        checked={checked}
        disabled={disabled}
        onChange={(event) => onChange(event.target.checked)}
      />
    </chakra.label>
  );
}
