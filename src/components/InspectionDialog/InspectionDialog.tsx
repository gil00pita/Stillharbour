import { useEffect, useState, type ReactNode } from 'react';
import { Box, useRecipe, Heading, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { inspectionDialogRecipe } from './InspectionDialog.recipe';

export function InspectionDialog({
  title,
  children,
  onClose,
  footer,
  busy = false,
}: {
  title: string;
  children: ReactNode;
  onClose: () => void;
  footer?: ReactNode;
  busy?: boolean;
}) {
  const [element, setElement] = useState<HTMLDialogElement | null>(null);
  const styles = useRecipe({ recipe: inspectionDialogRecipe })();
  useEffect(() => {
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, [element]);
  function close() {
    if (busy) return;
    element?.close();
    onClose();
  }
  return (
    <Box asChild css={styles}>
      <dialog
        ref={setElement}
        className="inspection-dialog"
        aria-label={title}
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !event.defaultPrevented) {
            event.preventDefault();
            event.stopPropagation();
            close();
          }
        }}
        onCancel={(event) => {
          event.preventDefault();
          close();
        }}
      >
        <chakra.header>
          <Heading as="h2" unstyled>
            {title}
          </Heading>
          <Button disabled={busy} onClick={close}>
            Close
          </Button>
        </chakra.header>
        <Box className="inspection-content">{children}</Box>
        {footer && <chakra.footer className="inspection-dialog-footer">{footer}</chakra.footer>}
      </dialog>
    </Box>
  );
}
