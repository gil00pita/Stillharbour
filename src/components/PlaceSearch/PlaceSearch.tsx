import { Box, Button, Field, Input, Popover, Portal, Stack, Text } from '@chakra-ui/react';
import { useId, useRef, useState } from 'react';
import type { PlaceSummary } from '../../bindings';
import { usePlaceLookup } from './use-place-lookup';
import { PlaceResults, PlaceStatus } from './components/PlaceResults';

export function PlaceSearch({
  selectedId,
  onChange,
  value,
  onSearch,
  disabled = false,
}: {
  selectedId?: string | null | undefined;
  onChange: (place: PlaceSummary | null) => void;
  value?: string;
  onSearch?: (text: string) => void;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');
  const text = value ?? draft;
  const input = useRef<HTMLInputElement>(null);
  const suppressFocus = useRef(false);
  function close() {
    setOpen(false);
    suppressFocus.current = true;
    input.current?.focus();
    suppressFocus.current = false;
  }
  const id = useId();
  const { current, error, selected, retry } = usePlaceLookup(open, text, selectedId);
  const showSuggestions =
    open &&
    !disabled &&
    (Boolean(selectedId) ||
      Boolean(error) ||
      (text.trim().length >= 2 && (current?.matches.length ?? 0) > 0));

  function choose(place: PlaceSummary | null) {
    if (disabled) return;
    onChange(place);
    setDraft('');
    close();
  }
  return (
    <Popover.Root
      open={showSuggestions}
      onOpenChange={(event) => setOpen(event.open)}
      autoFocus={false}
      positioning={{ placement: 'bottom-start' }}
      lazyMount
      unmountOnExit
    >
      <Popover.Anchor asChild>
        <Box className="fixed-search unified-search">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <circle cx="10.5" cy="10.5" r="6.5" stroke="currentColor" strokeWidth="1.5" />
            <path
              d="m16 16 4.5 4.5"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </svg>
          <Field.Root>
            <Field.Label className="screen-reader-status">
              Search filenames, dates or places
            </Field.Label>
            <Input
              ref={input}
              type="search"
              aria-label="Search filenames, dates or places"
              maxLength={256}
              value={text}
              disabled={disabled}
              placeholder="Search filenames, dates or places…"
              title="Search a filename, DD-MM-YYYY / YYYY-MM-DD, city or country"
              aria-expanded={showSuggestions}
              aria-controls={showSuggestions ? id : undefined}
              aria-haspopup="dialog"
              onFocus={() => {
                if (!suppressFocus.current) setOpen(true);
              }}
              onClick={() => setOpen(true)}
              onKeyDown={(event) => {
                if (event.key === 'Escape') {
                  setOpen(false);
                  event.stopPropagation();
                }
                if (event.key === 'ArrowDown') {
                  event.preventDefault();
                  document.getElementById(id)?.querySelector<HTMLButtonElement>('button')?.focus();
                }
              }}
              onChange={(event) => {
                setDraft(event.target.value);
                onSearch?.(event.target.value);
                setOpen(true);
              }}
            />
          </Field.Root>
          {selectedId && (
            <Button
              type="button"
              className="place-search-chip"
              onClick={() => choose(null)}
              disabled={disabled}
              title="Clear place filter"
            >
              {selected?.label ?? (error ? 'Place unavailable' : 'Selected place')} · Clear
            </Button>
          )}
        </Box>
      </Popover.Anchor>
      <Portal>
        <Popover.Positioner>
          <Popover.Content id={id} width="sm" maxW="100vw" maxH="64" overflowY="auto">
            <Popover.Body>
              <Stack gap="3">
                <Popover.Title>Find photos by place</Popover.Title>
                <Text m="0" fontSize="sm">
                  Filename and date results update as you type. Choose a place to search indexed GPS
                  instead.
                </Text>
                <Text m="0" fontSize="sm">
                  Offline · Only photos with indexed GPS coordinates can match.
                </Text>
                {selectedId && (
                  <Text m="0" fontSize="sm">
                    Selected:{' '}
                    {selected?.label ?? (error ? 'Place details unavailable' : 'Loading place…')}
                  </Text>
                )}
                {selectedId && (
                  <Button type="button" onClick={() => choose(null)} disabled={disabled}>
                    Clear place filter
                  </Button>
                )}
                <PlaceStatus error={error} current={current} text={text} retry={retry} />
                {current && !error && (
                  <PlaceResults reply={current} disabled={disabled} choose={choose} />
                )}
                <Button
                  type="button"
                  onClick={() => {
                    close();
                  }}
                >
                  Close place search
                </Button>
              </Stack>
            </Popover.Body>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
