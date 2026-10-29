import { useId, useState } from 'react';

export function TagInput({
  name,
  label,
  placeholder,
  values,
  onChange,
  disabled = false,
}: {
  name: string;
  label: string;
  placeholder: string;
  values: string[];
  onChange: (values: string[]) => void;
  disabled?: boolean;
}) {
  const id = useId();
  const [draft, setDraft] = useState('');
  function add(entries: string[]) {
    const additions = entries.map((value) => value.trim()).filter(Boolean);
    if (additions.length) onChange(Array.from(new Set([...values, ...additions])));
    setDraft('');
  }
  return (
    <div className="tag-field" data-field={name}>
      <label htmlFor={id}>{label}</label>
      <div className={`tag-editor ${disabled ? 'is-disabled' : ''}`}>
        {values.length > 0 && (
          <ul className="tag-values" aria-label={`${label} values`}>
            {values.map((value) => (
              <li key={value}>
                <bdi>{value}</bdi>
                <button
                  type="button"
                  disabled={disabled}
                  aria-label={`Remove ${value} from ${label}`}
                  onClick={() => onChange(values.filter((item) => item !== value))}
                >
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
                    <path
                      d="m4 4 8 8M12 4l-8 8"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                    />
                  </svg>
                </button>
              </li>
            ))}
          </ul>
        )}
        <input
          id={id}
          name={name}
          value={draft}
          placeholder={values.length ? 'Add another…' : placeholder}
          disabled={disabled}
          autoComplete="off"
          spellCheck={false}
          aria-description="Press Enter or Tab to add a value. Paste multiple values on separate lines. Backspace on an empty input edits the last value."
          onChange={(event) => setDraft(event.target.value)}
          onBlur={() => add([draft])}
          onKeyDown={(event) => {
            if (event.nativeEvent.isComposing) return;
            if (event.key === 'Enter') {
              event.preventDefault();
              add([draft]);
            } else if (event.key === 'Tab' && draft.trim()) add([draft]);
            else if (event.key === 'Backspace' && !draft && values.length) {
              event.preventDefault();
              setDraft(values.at(-1) ?? '');
              onChange(values.slice(0, -1));
            }
          }}
          onPaste={(event) => {
            const text = event.clipboardData.getData('text');
            if (!/[\r\n]/.test(text)) return;
            event.preventDefault();
            const input = event.currentTarget;
            add(
              [
                `${draft.slice(0, input.selectionStart ?? draft.length)}${text}${draft.slice(input.selectionEnd ?? draft.length)}`,
              ].flatMap((value) => value.split(/\r?\n|\r/)),
            );
          }}
        />
      </div>
    </div>
  );
}
