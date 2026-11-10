import { Heading, HStack, chakra } from '@chakra-ui/react';
import { useEffect, useRef, useState, type FormEvent } from 'react';
import { Button } from '@/components/Button';
import { BrowserControls } from '@/components/BrowserControls';
import { ThumbnailScale } from '@/components/ThumbnailScale';
import type { GalleryDisplay } from '@/components/FilterPanel';
import type { ViewMode } from '../../library/model';

export function DisplayPanel({
  mode,
  display,
  busy,
  onApply,
  onClose,
}: {
  mode: ViewMode;
  display: GalleryDisplay;
  busy: boolean;
  onApply: (mode: ViewMode, display: GalleryDisplay) => void;
  onClose: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [draftMode, setMode] = useState(mode);
  const [draftDisplay, setDisplay] = useState(display);
  useEffect(() => {
    const element = dialog.current;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    if (element && !element.open) element.showModal();
    return () => {
      element?.close();
      if (opener?.isConnected) opener.focus({ preventScroll: true });
    };
  }, []);
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy) return;
    onApply(draftMode, draftDisplay);
    onClose();
  }
  return (
    <chakra.dialog
      ref={dialog}
      id="library-display-panel"
      aria-labelledby="display-panel-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
      css={{
        position: 'fixed',
        inset: '0 0 0 auto',
        margin: 0,
        width: 'min(380px, 92vw)',
        height: '100dvh',
        maxHeight: 'none',
        maxWidth: 'none',
        padding: 0,
        border: 0,
        borderLeft: '1px solid {colors.border}',
        background: 'bg.panel',
        color: 'fg',
        '&::backdrop': { background: '#0005' },
      }}
    >
      <chakra.form
        onSubmit={submit}
        display="flex"
        flexDirection="column"
        height="full"
        minHeight="0"
      >
        <HStack
          as="header"
          justify="space-between"
          gap="3"
          p="5"
          bg="bg.toolbar"
          borderBottom="1px solid {colors.border}"
        >
          <Heading as="h2" id="display-panel-title" fontSize="lg">
            Display options
          </Heading>
          <Button
            type="button"
            aria-label="Close display options"
            disabled={busy}
            onClick={onClose}
          >
            Close
          </Button>
        </HStack>
        <chakra.fieldset
          disabled={busy}
          flex="1"
          minWidth="0"
          minHeight="0"
          overflowY="auto"
          border="0"
          margin="0"
          p="5"
          css={{
            '& .view-switch': { margin: 0, gap: '2' },
            '& .thumbnail-control': { width: 'full', margin: 0 },
          }}
        >
          <BrowserControls
            mode={draftMode}
            size={draftDisplay.size}
            fit={draftDisplay.fit}
            onMode={setMode}
            onSize={(size) => setDisplay((previous) => ({ ...previous, size }))}
            onFit={(fit) => setDisplay((previous) => ({ ...previous, fit }))}
            sizeControl={
              <ThumbnailScale
                size={draftDisplay.size}
                disabled={busy}
                onChange={(size) => setDisplay((previous) => ({ ...previous, size }))}
              />
            }
          />
        </chakra.fieldset>
        <HStack
          as="footer"
          justify="flex-end"
          gap="2"
          p="5"
          bg="bg.toolbar"
          borderTop="1px solid {colors.border}"
        >
          <Button type="button" disabled={busy} onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" className="primary" disabled={busy}>
            Apply display
          </Button>
        </HStack>
      </chakra.form>
    </chakra.dialog>
  );
}
