import { Box, Dialog, Field, Flex, Portal, Text, chakra } from '@chakra-ui/react';
import { useRef, useState } from 'react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import type { OrganizationTemplate } from '../../../bindings';
import type { LayoutTab } from '@/components/OrganizationPresets';
import { removeSessionPreset, saveSessionPreset, type SessionPreset } from '../structure-rules';
import { filenameChoice } from '../structure-rules';
import { structureDialogStyle } from '../StructureFormatter.recipe';

function description(preset: SessionPreset) {
  const category = {
    date: 'Date',
    location: 'Location',
    tags: 'Tags',
    camera: 'Camera',
    custom: 'Custom',
  }[preset.category];
  const naming = {
    original: 'Original filenames',
    date_original: 'Date + original',
    sequence: 'Sequence numbers',
    custom: 'Custom filename',
  }[filenameChoice(preset.template.pattern)];
  const missing = {
    skip: 'Skip missing dates',
    undated: 'Undated folder',
    block: 'Block missing dates',
  }[preset.template.missingDate];
  return `${category} · ${naming} · ${missing}`;
}

export function SessionPresetsDialog({
  mode,
  catalog,
  template,
  category,
  presets,
  onPresetsChange,
  onLoad,
  onClose,
}: {
  mode: 'save' | 'load';
  catalog: string;
  template: OrganizationTemplate;
  category: LayoutTab;
  presets: SessionPreset[];
  onPresetsChange: (presets: SessionPreset[]) => void;
  onLoad: (preset: SessionPreset) => void;
  onClose: () => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const close = useRef<HTMLButtonElement>(null);
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  return (
    <Dialog.Root
      open
      onOpenChange={({ open }) => {
        if (!open) onClose();
      }}
      initialFocusEl={() => (mode === 'save' ? input.current : close.current)}
    >
      <Portal>
        <Dialog.Backdrop />
        <Dialog.Positioner>
          <Dialog.Content css={structureDialogStyle}>
            <Dialog.Header>
              <Dialog.Title>
                {mode === 'save' ? 'Save session preset' : 'Saved presets'}
              </Dialog.Title>
              <Dialog.Description>
                Folder and filename rules for this app session and catalog. Presets are cleared when
                the app reloads.
              </Dialog.Description>
            </Dialog.Header>
            <Dialog.Body>
              {mode === 'save' ? (
                <chakra.form
                  id="save-structure-preset"
                  onSubmit={(event) => {
                    event.preventDefault();
                    try {
                      onPresetsChange(
                        saveSessionPreset(catalog, {
                          id: crypto.randomUUID(),
                          name,
                          template,
                          category,
                        }),
                      );
                      onClose();
                    } catch (cause) {
                      setError(
                        cause instanceof Error ? cause.message : 'Could not save the preset.',
                      );
                    }
                  }}
                >
                  <Field.Root required invalid={Boolean(error)}>
                    <Field.Label>Preset name</Field.Label>
                    <Input
                      ref={input}
                      required
                      maxLength={80}
                      value={name}
                      placeholder="For example, My photo archive"
                      onChange={(event) => {
                        setName(event.target.value);
                        setError(null);
                      }}
                    />
                    <Field.ErrorText>{error}</Field.ErrorText>
                  </Field.Root>
                </chakra.form>
              ) : presets.length ? (
                <Box display="grid" gap="3">
                  {presets.map((preset) => (
                    <Flex key={preset.id} gap="3" align="center">
                      <Button
                        type="button"
                        className="structure-session-load"
                        aria-label={`Load preset ${preset.name}`}
                        onClick={() => onLoad(preset)}
                      >
                        <Text as="span">{preset.name}</Text>
                        <Text as="small">{description(preset)}</Text>
                      </Button>
                      <Button
                        type="button"
                        aria-label={`Remove preset ${preset.name}`}
                        onClick={() => onPresetsChange(removeSessionPreset(catalog, preset.id))}
                      >
                        Remove
                      </Button>
                    </Flex>
                  ))}
                </Box>
              ) : (
                <Text>No session presets yet. Save your current rules to reuse them.</Text>
              )}
            </Dialog.Body>
            <Dialog.Footer>
              <Button type="button" ref={close} onClick={onClose}>
                {mode === 'save' ? 'Cancel' : 'Close'}
              </Button>
              {mode === 'save' && (
                <Button
                  type="submit"
                  form="save-structure-preset"
                  className="primary"
                  css={{
                    marginTop: '28px',
                    border: '0',
                    background: '{colors.brand.solid}',
                    color: '{colors.brand.contrast}',
                    fontWeight: '600',
                    fontSize: '13px',
                    lineHeight: '20px',
                    borderRadius: 'var(--control-radius)',
                    padding: '12px 24px',
                    minHeight: '44px',
                    transition: 'background-color 140ms var(--ui-ease)',
                    '&:hover:enabled': { background: '{colors.brand.hover}' },
                  }}
                  disabled={!name.trim()}
                >
                  Save preset
                </Button>
              )}
            </Dialog.Footer>
          </Dialog.Content>
        </Dialog.Positioner>
      </Portal>
    </Dialog.Root>
  );
}
