import { Box, Field, Input } from '@chakra-ui/react';

export function SearchField({
  label,
  placeholder,
  value,
  onChange,
  disabled = false,
  maxLength,
}: {
  label: string;
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
  maxLength?: number;
}) {
  return (
    <Field.Root
      className="fixed-search"
      css={{
        flex: '1',
        minWidth: '80px',
        maxWidth: '360px',
        position: 'relative',
        '& > svg': {
          position: 'absolute',
          zIndex: '1',
          inset: '10px auto auto 12px',
          color: '{colors.fg.muted}',
          pointerEvents: 'none',
        },
        '&:has(> svg) input': { paddingInlineStart: '38px' },
        '& input': { width: '100%' },
      }}
      minW="0"
      flex="1 1 0%"
    >
      <Field.Label
        className="screen-reader-status"
        css={{
          position: 'absolute',
          width: '1px',
          height: '1px',
          overflow: 'hidden',
          clipPath: 'inset(50%)',
        }}
      >
        {label}
      </Field.Label>
      <Box
        className="search-input-frame"
        css={{
          display: 'flex',
          alignItems: 'center',
          gap: '8px',
          width: '100%',
          minWidth: '0',
          height: 'var(--control-height)',
          paddingInline: '8px',
          border: '1px solid {colors.border.control}',
          borderRadius: 'var(--control-radius)',
          background: '{colors.bg.input}',
          '& > svg': { flexShrink: '0', color: '{colors.fg.muted}', pointerEvents: 'none' },
          '& .search-text-input': {
            flex: '1',
            minWidth: '0',
            minHeight: '0',
            height: '100%',
            padding: '0 4px',
            border: '0',
            borderRadius: '0',
            background: 'transparent',
          },
          '&:focus-within': { outline: '2px solid {colors.brand.solid}', outlineOffset: '2px' },
          '& .search-text-input:focus-visible': { outline: 'none' },
        }}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="10.5" cy="10.5" r="6.5" stroke="currentColor" strokeWidth="1.5" />
          <path d="m16 16 4.5 4.5" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
        <Input
          className="search-text-input"
          type="search"
          maxLength={maxLength}
          placeholder={placeholder}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        />
      </Box>
    </Field.Root>
  );
}
