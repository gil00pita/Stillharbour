import {
  Box,
  Button,
  Field,
  IconButton,
  Input,
  Popover,
  Portal,
  Stack,
  Tag,
  Text,
} from '@chakra-ui/react';
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
  filterScope,
}: {
  selectedId?: string | null | undefined;
  onChange: (place: PlaceSummary | null) => void;
  value?: string;
  onSearch?: (text: string) => void;
  disabled?: boolean;
  filterScope?: { label: string; onClear: () => void } | undefined;
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
        <Box
          className="fixed-search unified-search"
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
            '& .place-search-chip': {
              fontSize: '12px',
              minHeight: '28px',
              marginTop: '4px',
              maxWidth: '100%',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            },
          }}
        >
          <Field.Root>
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
              Search filenames, dates or places
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
                '&:focus-within': {
                  outline: '2px solid {colors.brand.solid}',
                  outlineOffset: '2px',
                },
                '& .search-text-input:focus-visible': { outline: 'none' },
              }}
              display="flex"
              alignItems="center"
              gap="2"
              width="full"
              minW="0"
              minH="control"
              px="2"
              bg="bg.input"
              border="1px solid {colors.border.control}"
              borderRadius="control"
            >
              <Box asChild flexShrink="0" color="fg.muted">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <circle cx="10.5" cy="10.5" r="6.5" stroke="currentColor" strokeWidth="1.5" />
                  <path
                    d="m16 16 4.5 4.5"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                  />
                </svg>
              </Box>
              {filterScope && (
                <Tag.Root
                  unstyled
                  className="search-scope-tag"
                  display="inline-flex"
                  alignItems="center"
                  gap="1"
                  px="1"
                  minW="0"
                  maxW="60%"
                  flexShrink="1"
                  borderRadius="control"
                  bg="brand.selected"
                  color="fg"
                  fontSize="xs"
                  title={filterScope.label}
                >
                  <Tag.Label unstyled truncate>
                    {filterScope.label}
                  </Tag.Label>
                  <Tag.EndElement unstyled display="inline-flex" flexShrink="0">
                    <Tag.CloseTrigger unstyled asChild>
                      <IconButton
                        type="button"
                        disabled={disabled}
                        aria-label={`Clear ${filterScope.label} filter`}
                        title={`Remove ${filterScope.label} filter`}
                        minH="0"
                        px="0"
                        py="0"
                        boxSize="calc({sizes.control} - {spacing.3})"
                        bg="transparent"
                        onClick={() => {
                          filterScope.onClear();
                          close();
                        }}
                      >
                        <svg
                          width="12"
                          height="12"
                          viewBox="0 0 16 16"
                          fill="none"
                          aria-hidden="true"
                        >
                          <path
                            d="m4 4 8 8m0-8-8 8"
                            stroke="currentColor"
                            strokeWidth="1.5"
                            strokeLinecap="round"
                          />
                        </svg>
                      </IconButton>
                    </Tag.CloseTrigger>
                  </Tag.EndElement>
                </Tag.Root>
              )}
              <Input
                ref={input}
                className="search-text-input"
                flex="1"
                minW="0"
                minH="0"
                border="0"
                borderRadius="0"
                bg="transparent"
                px="1"
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
                    document
                      .getElementById(id)
                      ?.querySelector<HTMLButtonElement>('button')
                      ?.focus();
                  }
                }}
                onChange={(event) => {
                  setDraft(event.target.value);
                  onSearch?.(event.target.value);
                  setOpen(true);
                }}
              />
            </Box>
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
