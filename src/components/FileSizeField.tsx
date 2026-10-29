import { useId, useState } from 'react';

const units = [
  { label: 'KB', digits: 3, factor: 1_000n },
  { label: 'MB', digits: 6, factor: 1_000_000n },
  { label: 'GB', digits: 9, factor: 1_000_000_000n },
] as const;
type SizeUnit = (typeof units)[number];

function parseSize(amount: string, unit: SizeUnit): { bytes: string | null; error: string } {
  const value = amount.trim();
  if (!value) return { bytes: null, error: '' };
  if (value.length > 64 || !/^(?:\d+(?:\.\d*)?|\.\d+)$/.test(value))
    return { bytes: null, error: 'Enter a nonnegative number, such as 1.5.' };
  const [whole = '', fraction = ''] = value.split('.');
  const divisor = 10n ** BigInt(fraction.length);
  const scaled = (BigInt(whole || '0') * divisor + BigInt(fraction || '0')) * unit.factor;
  if (scaled % divisor !== 0n)
    return { bytes: null, error: 'Use a size that equals a whole number of bytes.' };
  const bytes = scaled / divisor;
  if (bytes > 18_446_744_073_709_551_615n) return { bytes: null, error: 'This size is too large.' };
  return { bytes: bytes.toString(), error: '' };
}

function initialSize(bytes: string | null) {
  if (!bytes) return { amount: '', unit: units[1] };
  // Existing byte thresholds must survive opening and saving without rounding.
  const value = BigInt(bytes);
  const unit = [...units].reverse().find((unit) => value >= unit.factor) ?? units[0];
  const fraction = (value % unit.factor).toString().padStart(unit.digits, '0').replace(/0+$/, '');
  return {
    amount: `${value / unit.factor}${fraction ? `.${fraction}` : ''}`,
    unit,
  };
}

export function FileSizeField({
  name,
  label,
  initialBytes,
  disabled,
  onChange,
}: {
  name: string;
  label: string;
  initialBytes: string | null;
  disabled: boolean;
  onChange: (bytes: string | null) => void;
}) {
  const id = useId();
  const [draft, setDraft] = useState(() => initialSize(initialBytes));
  const { error } = parseSize(draft.amount, draft.unit);
  function update(amount: string, unit: SizeUnit) {
    setDraft({ amount, unit });
    const result = parseSize(amount, unit);
    if (!result.error) onChange(result.bytes);
  }
  return (
    <div className="file-size-field">
      <label htmlFor={id}>{label}</label>
      <div className="file-size-controls">
        <input
          id={id}
          name={name}
          inputMode="decimal"
          maxLength={64}
          placeholder="No limit"
          disabled={disabled}
          value={draft.amount}
          aria-invalid={Boolean(error)}
          aria-describedby={error ? `${id}-error` : undefined}
          ref={(input) => {
            input?.setCustomValidity(error);
          }}
          onChange={(event) => update(event.target.value, draft.unit)}
        />
        <select
          aria-label={`${label} unit`}
          disabled={disabled}
          value={draft.unit.label}
          onChange={(event) => {
            const unit = units.find((unit) => unit.label === event.target.value);
            if (unit) update(draft.amount, unit);
          }}
        >
          {units.map((unit) => (
            <option key={unit.label} value={unit.label}>
              {unit.label}
            </option>
          ))}
        </select>
      </div>
      {error && (
        <p id={`${id}-error`} className="field-error">
          {error}
        </p>
      )}
    </div>
  );
}
