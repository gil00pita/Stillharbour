import { Box, Button, Field, Input, Popover, Portal, Stack, Text } from '@chakra-ui/react';
import { useRef, useState } from 'react';
import type { PlaceSummary } from '../../bindings';
import { usePlaceLookup } from './use-place-lookup';
import { PlaceResults, PlaceStatus } from './components/PlaceResults';

export function PlaceSearch({
  selectedId,
  onChange,
  disabled = false,
}: {
  selectedId?: string | null | undefined;
  onChange: (place: PlaceSummary | null) => void;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState('');
  const input = useRef<HTMLInputElement>(null);
  const { current, error, selected, retry } = usePlaceLookup(open, text, selectedId);
  function choose(place: PlaceSummary | null) {
    if (disabled) return;
    onChange(place);
    setOpen(false);
  }
  return (
    <Popover.Root
      open={open}
      onOpenChange={(event) => setOpen(event.open)}
      initialFocusEl={() => input.current}
      positioning={{ placement: 'bottom-end' }}
      lazyMount
      unmountOnExit
    >
      <Popover.Trigger asChild>
        <Button
          type="button"
          variant="outline"
          aria-label="Search places"
          title={selected?.label ?? 'Search offline cities and countries'}
          disabled={disabled}
          flexShrink="0"
          px="3"
        >
          Places{selectedId ? ' · 1' : ''}
        </Button>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content width="sm" maxW="100vw" maxH="90vh" overflowY="auto">
            <Popover.Body>
              <Stack gap="3">
                <Popover.Title>Find photos by place</Popover.Title>
                <Field.Root>
                  <Field.Label>City or country</Field.Label>
                  <Input
                    ref={input}
                    type="search"
                    maxLength={128}
                    value={text}
                    placeholder="For example, London or Portugal"
                    onChange={(event) => setText(event.target.value)}
                  />
                  <Field.HelperText>
                    Offline · Only photos with indexed GPS coordinates can match.
                  </Field.HelperText>
                </Field.Root>
                {selectedId && (
                  <Box>
                    <Text m="0" fontSize="sm">
                      Selected:{' '}
                      {selected?.label ?? (error ? 'Place details unavailable' : 'Loading place…')}
                    </Text>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => choose(null)}
                      disabled={disabled}
                    >
                      Clear place filter
                    </Button>
                  </Box>
                )}
                <PlaceStatus error={error} current={current} text={text} retry={retry} />
                {current && !error && (
                  <PlaceResults reply={current} disabled={disabled} choose={choose} />
                )}
                <Popover.CloseTrigger aria-label="Close place search" asChild>
                  <Button type="button" variant="outline">
                    Close place search
                  </Button>
                </Popover.CloseTrigger>
              </Stack>
            </Popover.Body>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
