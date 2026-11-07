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
    <Field.Root className="fixed-search" minW="0" flex="1 1 0%">
      <Field.Label className="screen-reader-status">{label}</Field.Label>
      <Box className="search-input-frame">
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
