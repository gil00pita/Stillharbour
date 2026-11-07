import { Box, Field, IconButton, List, Portal, Text, useRecipe } from '@chakra-ui/react';
import { useId, useRef, useState } from 'react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { structurePathBuilderRecipe, chipDragPreviewStyle } from './StructurePathBuilder.recipe';
import { pathParts, withFolders, folderChoices, folderPartLabel } from './path-parts';
import { reorderFolders } from './reorder';
import { useFolderReorder } from './use-folder-reorder';

function ChipIcon({ kind }: { kind: 'grip' | 'remove' }) {
  return (
    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      {kind === 'grip' ? (
        <path
          d="M5 3h.01M11 3h.01M5 8h.01M11 8h.01M5 13h.01M11 13h.01"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
        />
      ) : (
        <path
          d="m4 4 8 8M12 4l-8 8"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      )}
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
  const { container, interaction, announcement, handleProps } = useFolderReorder(
    pattern,
    folders,
    disabled,
    (from, to) => change(reorderFolders(folders, from, to)),
  );
  const occurrences = new Map<string, number>();
  return (
    <Box css={styles} className="structure-path-builder">
      <Field.Root disabled={disabled} unstyled display="grid" gap="2" minWidth="0">
        <Field.Label htmlFor={id}>Folder levels</Field.Label>
        <Box className="structure-path-input" ref={container}>
          <List.Root unstyled className="structure-path-chips" aria-label="Folder levels in order">
            {folders.map((part, index) => {
              const occurrence = (occurrences.get(part) ?? 0) + 1;
              occurrences.set(part, occurrence);
              return (
                <List.Item
                  unstyled
                  className="structure-path-chip"
                  key={`${part}:${occurrence}`}
                  data-picked={interaction?.from === index || undefined}
                  data-dragging={
                    (interaction?.kind === 'drag' && interaction.from === index) || undefined
                  }
                  data-drop={
                    interaction?.to === index && interaction.from !== index
                      ? interaction.from < index
                        ? 'after'
                        : 'before'
                      : undefined
                  }
                >
                  <Button
                    type="button"
                    className="structure-chip-handle"
                    disabled={disabled}
                    data-folder-index={index}
                    aria-label={`Reorder ${label(part)} level ${index + 1}`}
                    aria-describedby={`${id}-reorder-help`}
                    aria-pressed={interaction?.from === index}
                    title="Drag to reorder, or click to pick up and place"
                    {...handleProps(index)}
                  >
                    <ChipIcon kind="grip" />
                    <Text as="span">{label(part)}</Text>
                  </Button>
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
        <Field.HelperText
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
        >
          Chips run from outermost folder to innermost. Search predefined parts, or type a literal
          folder name. Drag chips to reorder, or click a chip and then its destination. Backspace
          removes the last level.
        </Field.HelperText>
        <Text
          id={`${id}-reorder-help`}
          className="screen-reader-status"
          css={{
            position: 'absolute',
            width: '1px',
            height: '1px',
            overflow: 'hidden',
            clipPath: 'inset(50%)',
          }}
        >
          Space or Enter picks up a folder level. Arrow keys move its destination; Home and End
          choose the first and last level. Enter places it. Escape cancels.
        </Text>
        <Text
          role="status"
          aria-label="Folder order"
          className="screen-reader-status"
          css={{
            position: 'absolute',
            width: '1px',
            height: '1px',
            overflow: 'hidden',
            clipPath: 'inset(50%)',
          }}
          aria-live="polite"
          aria-atomic="true"
        >
          {announcement}
        </Text>
      </Field.Root>
      {interaction?.ghost && (
        <Portal>
          <Box
            css={chipDragPreviewStyle}
            aria-hidden="true"
            data-folder-drag-preview=""
            style={{
              transform: `translate(${interaction.ghost.x}px, ${interaction.ghost.y}px)`,
              width: interaction.ghost.width,
            }}
          >
            <ChipIcon kind="grip" />
            <Text as="span">{label(folders[interaction.from] ?? '')}</Text>
          </Box>
        </Portal>
      )}
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
        <Text
          className="help"
          css={{
            fontSize: '13px',
            color: '{colors.fg.muted}',
            lineHeight: '20px',
            margin: '8px 0',
          }}
          role="status"
        >
          No matching folder part. Use a predefined part or a folder name without path separators or
          reserved characters.
        </Text>
      )}
      <Text
        className="help"
        css={{ fontSize: '13px', color: '{colors.fg.muted}', lineHeight: '20px', margin: '8px 0' }}
      >
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
