import { Box, Button, IconButton, Popover, Portal, Stack, Text } from '@chakra-ui/react';
import { useState } from 'react';
import { formatScanDate } from '../../utils/format-scan-date';

export type UtilityVersion = { id: string; date: string; label: string; state: string };

export function UtilityHistory({
  versions,
  selectedId,
  disabled,
  onSelect,
}: {
  versions: UtilityVersion[];
  selectedId?: string | undefined;
  disabled?: boolean;
  onSelect: (id: string) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <Popover.Root
      open={open}
      onOpenChange={(event) => setOpen(event.open)}
      positioning={{ placement: 'bottom-end' }}
      lazyMount
      unmountOnExit
    >
      <Popover.Trigger asChild>
        <IconButton
          aria-label="Scan history"
          title="Scan history"
          disabled={disabled}
          variant="outline"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M3 11a9 9 0 1 1 2.5 7M3 5v6h6m3-4v5l3 2" />
          </svg>
        </IconButton>
      </Popover.Trigger>
      <Portal>
        <Popover.Positioner>
          <Popover.Content
            bg="bg.panel"
            color="fg"
            borderWidth="1px"
            borderColor="border.control"
            borderRadius="panel"
            p="4"
            width="sm"
            maxW="calc(100vw - 32px)"
            maxH="60vh"
            overflowY="auto"
          >
            <Popover.Title fontWeight="semibold">Scan history</Popover.Title>
            <Text color="fg.muted" fontSize="sm">
              Saved observations. Earlier versions do not approve file changes.
            </Text>
            <Stack gap="2" aria-label="Saved scan versions">
              {versions.length === 0 && <Text color="fg.muted">Your scans will appear here.</Text>}
              {versions.map((version) => (
                <Button
                  key={version.id}
                  variant="outline"
                  height="auto"
                  py="3"
                  textAlign="left"
                  justifyContent="start"
                  whiteSpace="normal"
                  aria-pressed={version.id === selectedId}
                  onClick={() => {
                    onSelect(version.id);
                    setOpen(false);
                  }}
                >
                  <Box>
                    <Text margin="0">{version.label}</Text>
                    <Text margin="0" color="fg.muted" fontSize="sm">
                      {formatScanDate(version.date)} · {version.state.replaceAll('_', ' ')}
                    </Text>
                  </Box>
                </Button>
              ))}
            </Stack>
          </Popover.Content>
        </Popover.Positioner>
      </Portal>
    </Popover.Root>
  );
}
