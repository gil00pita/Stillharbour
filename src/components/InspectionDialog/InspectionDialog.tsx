import { useEffect, useState, type ReactNode } from 'react';
import { Box, useRecipe } from '@chakra-ui/react';
import { Button } from '../ui/controls';
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
        onCancel={(event) => {
          event.preventDefault();
          close();
        }}
      >
        <header>
          <h2>{title}</h2>
          <Button disabled={busy} onClick={close}>
            Close
          </Button>
        </header>
        <div className="inspection-content">{children}</div>
        {footer && <footer className="inspection-dialog-footer">{footer}</footer>}
      </dialog>
    </Box>
  );
}
