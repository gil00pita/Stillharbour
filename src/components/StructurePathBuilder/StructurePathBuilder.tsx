import { Box, Field, IconButton, List, Text, useRecipe } from '@chakra-ui/react';
import { useId, useRef, useState } from 'react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { structurePathBuilderRecipe } from './StructurePathBuilder.recipe';
import { pathParts, withFolders, folderChoices, folderPartLabel } from './path-parts';

function ChipIcon({ kind }: { kind: 'left' | 'right' | 'remove' }) {
  return (
    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d={
          kind === 'remove'
            ? 'm4 4 8 8M12 4l-8 8'
            : kind === 'left'
              ? 'm10 3-5 5 5 5'
              : 'm6 3 5 5-5 5'
        }
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
export function StructurePathBuilder({
  pattern,
  disabled,
  onChange,
}: {
  pattern: string;
  disabled: boolean;
  onChange: (pattern: string) => void;
}) {
  const styles = useRecipe({ recipe: structurePathBuilderRecipe })();
  const id = useId();
  const input = useRef<HTMLInputElement>(null);
  const [draft, setDraft] = useState('');
  const [active, setActive] = useState(0);
  const [open, setOpen] = useState(true);
  const [focused, setFocused] = useState(false);
  const { folders, filename } = pathParts(pattern);
  const choices = folderChoices(draft);
  const current = Math.min(active, Math.max(0, choices.length - 1));
  const label = folderPartLabel;
  function change(parts: string[]) {
    onChange(withFolders(parts, filename));
  }
  function add(value: string) {
    if (disabled || folders.length >= 31) return;
    change([...folders, value]);
    setDraft('');
    setActive(0);
    input.current?.focus();
  }
  function move(index: number, delta: number) {
    const parts = [...folders];
    const destination = index + delta;
    const part = parts[index];
    if (part === undefined || destination < 0 || destination >= parts.length) return;
    parts.splice(index, 1);
    parts.splice(destination, 0, part);
    change(parts);
  }
  const occurrences = new Map<string, number>();
  return (
    <Box css={styles} className="structure-path-builder">
      <Field.Root disabled={disabled} unstyled display="grid" gap="2" minWidth="0">
        <Field.Label htmlFor={id}>Folder levels</Field.Label>
        <Box className="structure-path-input">
          <List.Root unstyled className="structure-path-chips" aria-label="Folder levels in order">
            {folders.map((part, index) => {
              const occurrence = (occurrences.get(part) ?? 0) + 1;
              occurrences.set(part, occurrence);
              return (
                <List.Item unstyled className="structure-path-chip" key={`${part}:${occurrence}`}>
                  <Text as="span">{label(part)}</Text>
                  <IconButton
                    type="button"
                    disabled={disabled || index === 0}
                    aria-label={`Move ${label(part)} level ${index + 1} earlier`}
                    onClick={() => move(index, -1)}
                  >
                    <ChipIcon kind="left" />
                  </IconButton>
                  <IconButton
                    type="button"
                    disabled={disabled || index === folders.length - 1}
                    aria-label={`Move ${label(part)} level ${index + 1} later`}
                    onClick={() => move(index, 1)}
                  >
                    <ChipIcon kind="right" />
                  </IconButton>
                  <IconButton
                    type="button"
                    disabled={disabled}
                    aria-label={`Remove ${label(part)} level ${index + 1}`}
                    onClick={() => {
                      change(folders.filter((_, i) => i !== index));
                      input.current?.focus();
                    }}
                  >
                    <ChipIcon kind="remove" />
                  </IconButton>
                </List.Item>
              );
            })}
          </List.Root>
          <Input
            id={id}
            ref={input}
            role="combobox"
            aria-autocomplete="list"
            aria-expanded={open && choices.length > 0}
            aria-controls={`${id}-options`}
            aria-activedescendant={
              focused && open && choices.length > 0 ? `${id}-option-${current}` : undefined
            }
            placeholder="Add Year, Country, Tag…"
            disabled={disabled || folders.length >= 31}
            value={draft}
            onFocus={() => {
              setOpen(true);
              setFocused(true);
            }}
            onBlur={() => setFocused(false)}
            onChange={(e) => {
              setDraft(e.target.value);
              setActive(0);
              setOpen(true);
            }}
            onKeyDown={(e) => {
              if (e.nativeEvent.isComposing) return;
              if (e.key === 'Escape') {
                e.preventDefault();
                setOpen(false);
              } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                setOpen(true);
                setActive((value) =>
                  Math.max(
                    0,
                    Math.min(choices.length - 1, value + (e.key === 'ArrowDown' ? 1 : -1)),
                  ),
                );
              } else if (e.key === 'Enter') {
                e.preventDefault();
                const choice = choices[current];
                if (open && choice) add(choice.value);
              } else if (e.key === 'Backspace' && !draft && folders.length > 0) {
                e.preventDefault();
                change(folders.slice(0, -1));
              }
            }}
          />
        </Box>
        <Field.HelperText className="help">
          Chips run from outermost folder to innermost. Search predefined parts, or type a literal
          folder name. Use the arrows to reorder; Backspace removes the last level.
        </Field.HelperText>
      </Field.Root>
      {open && (
        <Box
          as="ul"
          role="listbox"
          id={`${id}-options`}
          className="structure-part-options"
          aria-label="Available folder parts"
        >
          {choices.map((choice, index) => (
            <Box as="li" key={choice.value} role="presentation">
              <Button
                type="button"
                role="option"
                id={`${id}-option-${index}`}
                aria-selected={focused && index === current}
                disabled={disabled || folders.length >= 31}
                tabIndex={-1}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => add(choice.value)}
              >
                + {choice.label}
              </Button>
            </Box>
          ))}
        </Box>
      )}
      {open && draft && choices.length === 0 && (
        <Text className="help" role="status">
          No matching folder part. Use a predefined part or a folder name without path separators or
          reserved characters.
        </Text>
      )}
      <Text className="help">
        {folders.length}/31 folder levels. Filenames keep their original extension.
      </Text>
      <Field.Root unstyled disabled={disabled} display="grid" gap="2" minWidth="0">
        <Field.Label>Filename pattern</Field.Label>
        <Input
          value={filename}
          maxLength={255}
          onChange={(e) => onChange(withFolders(folders, e.target.value))}
        />
      </Field.Root>
      <Text className="structure-path-example" aria-label="Structure pattern">
        {folders.map(label).join(' / ')}
        {folders.length > 0 ? ' / ' : ''}
        {filename}
      </Text>
    </Box>
  );
}
